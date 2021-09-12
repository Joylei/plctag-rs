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



/*
 * This example reads from a large DINT array.  It creates many tags that each read from one element of the
 * array. It fires off all the tags at once and waits for them to complete the reads. In this case, it waits
 * a fixed amount of time and then tries to read the tags.
 */


#include <stdio.h>
#include <stdlib.h>
#include "../lib/libplctag.h"
#include "utils.h"


#define REQUIRED_VERSION 2,1,0
#define TAG_ATTRIBS "protocol=ab_eip&gateway=10.206.1.40&path=1,4&cpu=LGX&elem_type=DINT&elem_count=%d&name=TestBigArray[%d]"
#define NUM_TAGS  (30)
#define NUM_ELEMS (1000)
#define DATA_TIMEOUT (5000)

int main()
{
    int32_t tag[NUM_TAGS];
    int rc;
    int i;
    int64_t timeout = DATA_TIMEOUT + util_time_ms();
    int failed = 0;
    int done = 0;
    int64_t start = 0;
    int64_t end = 0;
    int num_elems_per_tag = NUM_ELEMS / NUM_TAGS;

    /* check the library version. */
    if(plc_tag_check_lib_version(REQUIRED_VERSION) != PLCTAG_STATUS_OK) {
        fprintf(stderr, "Required compatible library version %d.%d.%d not available!", REQUIRED_VERSION);
        exit(1);
    }

    do {
        /* create the tags */
        for(i=0; i< NUM_TAGS; i++) {
            char tmp_tag_path[256] = {0,};
            snprintf_platform(tmp_tag_path, sizeof tmp_tag_path,TAG_ATTRIBS, num_elems_per_tag, i);

            fprintf(stderr, "Attempting to create tag with attribute string '%s'\n",tmp_tag_path);

            tag[i]  = plc_tag_create(tmp_tag_path, 0);

            if(tag[i] < 0) {
                fprintf(stderr,"Error %s: could not create tag %d\n",plc_tag_decode_error(tag[i]), i);
                tag[i] = 0;
                failed = 1;
            }
        }

        /* did any tags fail? */
        if(failed) {
            rc = PLCTAG_ERR_CREATE;
            break;
        }

        /* wait for all the tags to complete creation. */
        do {
            done = 1;

            for(i=0; i < NUM_TAGS; i++) {
                rc = plc_tag_status(tag[i]);
                if(rc != PLCTAG_STATUS_OK) {
                    done = 0;
                }
            }

            if(!done) {
                util_sleep_ms(1);
            }
        } while(timeout > util_time_ms() && !done) ;

        if(!done) {
            fprintf(stderr, "Timeout waiting for tags to be ready!\n");
            rc = PLCTAG_ERR_TIMEOUT;
            break;
        }

        start = util_time_ms();

        /* get the data */
        for(i=0; i < NUM_TAGS; i++) {
            rc = plc_tag_read(tag[i], 0);

            if(rc != PLCTAG_STATUS_OK && rc != PLCTAG_STATUS_PENDING) {
                fprintf(stderr,"ERROR: Unable to read the data! Got error code %d: %s\n",rc, plc_tag_decode_error(rc));
                break;
            }
        }

        /* wait for all to finish */
        do {
            done = 1;

            for(i=0; i < NUM_TAGS; i++) {
                rc = plc_tag_status(tag[i]);
                if(rc != PLCTAG_STATUS_OK) {
                    done = 0;
                }
            }

            if(!done) {
                util_sleep_ms(1);
            }
        } while(timeout > util_time_ms() && !done);

        if(!done) {
            fprintf(stderr, "Timeout waiting for tags to finish reading!\n");
            rc = PLCTAG_ERR_TIMEOUT;
            break;
        }

        end = util_time_ms();

        /* get any data we can */
        for(i=0; i < NUM_TAGS; i++) {
            /* read complete! */
            fprintf(stderr,"Tag %d data[0]=%d\n",i,plc_tag_get_int32(tag[i],0));
        }
    } while(0);


    /* we are done */
    for(i=0; i < NUM_TAGS; i++) {
        if(tag[i] != 0) {
            plc_tag_destroy(tag[i]);
        }
    }

    if(rc == PLCTAG_STATUS_OK) {
        fprintf(stderr, "Read %d tags in %dms\n", NUM_TAGS, (int)(end - start));
    } else {
        fprintf(stderr, "Error found: %s\n", plc_tag_decode_error(rc));
    }

    return rc;
}
