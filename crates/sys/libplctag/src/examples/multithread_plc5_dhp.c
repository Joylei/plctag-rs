/***************************************************************************
 *   Copyright (C) 2020 by Kyle Hayes                                      *
 *   Author Kyle Hayes  kyle.hayes@gmail.com                               *
 *                                                                         *
 * This software is available under either the Mozilla Public License      *
 * version 2.0 or the GNU LGPL version 2 (or later) license, whichever     *
 * you choose.                                                             *
 *                                                                         *
 * MPL 2.0:                                                                *
 *                                                                         *
 *   This Source Code Form is subject to the terms of the Mozilla Public   *
 *   License, v. 2.0. If a copy of the MPL was not distributed with this   *
 *   file, You can obtain one at http://mozilla.org/MPL/2.0/.              *
 *                                                                         *
 *                                                                         *
 * LGPL 2:                                                                 *
 *                                                                         *
 *   This program is free software; you can redistribute it and/or modify  *
 *   it under the terms of the GNU Library General Public License as       *
 *   published by the Free Software Foundation; either version 2 of the    *
 *   License, or (at your option) any later version.                       *
 *                                                                         *
 *   This program is distributed in the hope that it will be useful,       *
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of        *
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
 *   GNU General Public License for more details.                          *
 *                                                                         *
 *   You should have received a copy of the GNU Library General Public     *
 *   License along with this program; if not, write to the                 *
 *   Free Software Foundation, Inc.,                                       *
 *   59 Temple Place - Suite 330, Boston, MA  02111-1307, USA.             *
 ***************************************************************************/

#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>
#include <stdint.h>
#include <sys/time.h>
#include "../lib/libplctag.h"
#include "utils.h"

#define REQUIRED_VERSION 2,1,0

#define TAG_PATH "protocol=ab_eip&gateway=10.206.1.39&path=1,2,A:27:1&cpu=plc5&elem_count=1&elem_size=2&name=N7:0&debug=4"
#define ELEM_COUNT 1
#define ELEM_SIZE 2
#define DATA_TIMEOUT 5000

#define MAX_THREADS (5)

/*
 * This test program creates a lot of threads that read the same tag in
 * the plc.  They all hit the exact same underlying tag data structure.
 * This tests, to some extent, whether the library can handle multi-threaded
 * access.
 *
 * This test does not stop by itself.  You need to kill it externally!
 */


/* global to cheat on passing it to threads. */
volatile int32_t tag = 0;
volatile int done = 0;

/*
 * Thread function.  Just read until killed.
 */

void *thread_func(void *data)
{
    int tid = (int)(intptr_t)data;
    int rc;
    float value;

    while(!done) {
        int64_t start;
        int64_t end;

        /* capture the starting time */
        start = util_time_ms();

        /* use do/while to allow easy exit without return */
        do {
            rc = plc_tag_lock(tag);
            if(rc != PLCTAG_STATUS_OK) {
                value = 1000;
                break; /* punt, no lock */
            }

            rc = plc_tag_read(tag, DATA_TIMEOUT);
            if(rc != PLCTAG_STATUS_OK) {
                value = 1001;
            } else {
                value =  plc_tag_get_float32(tag,0);

                /* increment the value */
                value = (float)(value > 500.0 ? 0.0 : value + 1.5);

                /* yes, we should be checking this return value too... */
                plc_tag_set_float32(tag, 0, value);

                /* write the value */
                rc = plc_tag_write(tag, DATA_TIMEOUT);
            }

            /* yes, we should look at the return value */
            plc_tag_unlock(tag);
        } while(0);

        end = util_time_ms();

        fprintf(stderr,"Thread %d got result %f with return code %s in %dms\n",tid,value,plc_tag_decode_error(rc),(int)(end-start));

        /* no short sleeps, this is a PLC5 */
        util_sleep_ms(10);
    }

    return NULL;
}


int main(int argc, char **argv)
{
    int rc = PLCTAG_STATUS_OK;
    pthread_t thread[MAX_THREADS];
    int num_threads;
    int thread_id = 0;

    /* check the library version. */
    if(plc_tag_check_lib_version(REQUIRED_VERSION) != PLCTAG_STATUS_OK) {
        fprintf(stderr, "Required compatible library version %d.%d.%d not available!", REQUIRED_VERSION);
        exit(1);
    }

    if(argc != 2) {
        fprintf(stderr,"ERROR: Must provide number of threads to run (between 1 and 300) argc=%d!\n",argc);
        return 0;
    }

    num_threads = (int)strtol(argv[1],NULL, 10);

    if(num_threads < 1 || num_threads > MAX_THREADS) {
        fprintf(stderr,"ERROR: %d (%s) is not a valid number. Must provide number of threads to run (between 1 and 300)!\n",num_threads, argv[1]);
        return 0;
    }

    /* create the tag */
    tag = plc_tag_create(TAG_PATH, DATA_TIMEOUT);

    /* everything OK? */
    if(tag < 0) {
        fprintf(stderr,"ERROR %s: Could not create tag!\n", plc_tag_decode_error(tag));
        return 0;
    }

    if((rc = plc_tag_status(tag)) != PLCTAG_STATUS_OK) {
        fprintf(stderr,"Error setting up tag internal state. %s\n", plc_tag_decode_error(rc));
        plc_tag_destroy(tag);
        return 0;
    }

    /* create the read threads */

    fprintf(stderr,"Creating %d threads.\n",num_threads);

    for(thread_id=0; thread_id < num_threads; thread_id++) {
        pthread_create(&thread[thread_id], NULL, thread_func, (void *)(intptr_t)thread_id);
    }

    /* wait until ^C */
    while(1) {
        util_sleep_ms(100);
    }

    done = 1;

    for(thread_id = 0; thread_id < num_threads; thread_id++) {
        pthread_join(thread[thread_id], NULL);
    }

    plc_tag_destroy(tag);

    return 0;
}
