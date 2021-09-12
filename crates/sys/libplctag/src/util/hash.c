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
 * This comes from Bob Jenkins's excellent site:
 *    http://burtleburtle.net/bob/c/lookup3.c
 * Thanks, Bob!
 */



#include <lib/libplctag.h>
#include <platform.h>
#include <util/debug.h>

/*
--------------------------------------------------------------------
lookup2.c, by Bob Jenkins, December 1996, Public Domain.
hash(), hash2(), hash3, and mix() are externally useful functions.
Routines to test the hash are included if SELF_TEST is defined.
You can use this free for any purpose.  It has no warranty.

Obsolete.  Use lookup3.c instead, it is faster and more thorough.
--------------------------------------------------------------------
*/
#define SELF_TEST

#include <stdio.h>
#include <stddef.h>
#include <stdlib.h>
typedef uint32_t ub4;   /* unsigned 4-byte quantities */
typedef uint8_t ub1;

#define hashsize(n) ((ub4)1<<(n))
#define hashmask(n) (hashsize(n)-1)

/*
--------------------------------------------------------------------
mix -- mix 3 32-bit values reversibly.
For every delta with one or two bit set, and the deltas of all three
  high bits or all three low bits, whether the original value of a,b,c
  is almost all zero or is uniformly distributed,
* If mix() is run forward or backward, at least 32 bits in a,b,c
  have at least 1/4 probability of changing.
* If mix() is run forward, every bit of c will change between 1/3 and
  2/3 of the time.  (Well, 22/100 and 78/100 for some 2-bit deltas.)
mix() was built out of 36 single-cycle latency instructions in a
  structure that could supported 2x parallelism, like so:
      a -= b;
      a -= c; x = (c>>13);
      b -= c; a ^= x;
      b -= a; x = (a<<8);
      c -= a; b ^= x;
      c -= b; x = (b>>13);
      ...
  Unfortunately, superscalar Pentiums and Sparcs can't take advantage
  of that parallelism.  They've also turned some of those single-cycle
  latency instructions into multi-cycle latency instructions.  Still,
  this is the fastest good hash I could find.  There were about 2^^68
  to choose from.  I only looked at a billion or so.
--------------------------------------------------------------------
*/
#define mix(a,b,c) \
    { \
        a -= b; a -= c; a ^= (c>>13); \
        b -= c; b -= a; b ^= (a<<8); \
        c -= a; c -= b; c ^= (b>>13); \
        a -= b; a -= c; a ^= (c>>12);  \
        b -= c; b -= a; b ^= (a<<16); \
        c -= a; c -= b; c ^= (b>>5); \
        a -= b; a -= c; a ^= (c>>3);  \
        b -= c; b -= a; b ^= (a<<10); \
        c -= a; c -= b; c ^= (b>>15); \
    }

/* same, but slower, works on systems that might have 8 byte ub4's */
#define mix2(a,b,c) \
    { \
        a -= b; a -= c; a ^= (c>>13); \
        b -= c; b -= a; b ^= (a<< 8); \
        c -= a; c -= b; c ^= ((b&0xffffffff)>>13); \
        a -= b; a -= c; a ^= ((c&0xffffffff)>>12); \
        b -= c; b -= a; b = (b ^ (a<<16)) & 0xffffffff; \
        c -= a; c -= b; c = (c ^ (b>> 5)) & 0xffffffff; \
        a -= b; a -= c; a = (a ^ (c>> 3)) & 0xffffffff; \
        b -= c; b -= a; b = (b ^ (a<<10)) & 0xffffffff; \
        c -= a; c -= b; c = (c ^ (b>>15)) & 0xffffffff; \
    }

/*
--------------------------------------------------------------------
hash() -- hash a variable-length key into a 32-bit value
  k     : the key (the unaligned variable-length array of bytes)
  len   : the length of the key, counting by bytes
  level : can be any 4-byte value
Returns a 32-bit value.  Every bit of the key affects every bit of
the return value.  Every 1-bit and 2-bit delta achieves avalanche.
About 36+6len instructions.

The best hash table sizes are powers of 2.  There is no need to do
mod a prime (mod is sooo slow!).  If you need less than 32 bits,
use a bitmask.  For example, if you need only 10 bits, do
  h = (h & hashmask(10));
In which case, the hash table should have hashsize(10) elements.

If you are hashing n strings (ub1 **)k, do it like this:
  for (i=0, h=0; i<n; ++i) h = hash( k[i], len[i], h);

By Bob Jenkins, 1996.  bob_jenkins@burtleburtle.net.  You may use this
code any way you wish, private, educational, or commercial.  It's free.

See http://burtleburtle.net/bob/hash/evahash.html
Use for hash table lookup, or anything where one collision in 2^32 is
acceptable.  Do NOT use for cryptographic purposes.
--------------------------------------------------------------------
*/

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable: 4061)
#endif

uint32_t hash( uint8_t *k, size_t length, uint32_t initval)
{
    uint32_t a,b,c,len;

    /* Set up the internal state */
    len = (uint32_t)length;
    a = b = 0x9e3779b9;  /* the golden ratio; an arbitrary value */
    c = initval;           /* the previous hash value */

    /*---------------------------------------- handle most of the key */
    while (len >= 12) {
        a += (uint32_t)((k[0] +((ub4)k[1]<<8) +((ub4)k[2]<<16) +((ub4)k[3]<<24)));
        b += (uint32_t)((k[4] +((ub4)k[5]<<8) +((ub4)k[6]<<16) +((ub4)k[7]<<24)));
        c += (uint32_t)((k[8] +((ub4)k[9]<<8) +((ub4)k[10]<<16)+((ub4)k[11]<<24)));
        mix(a,b,c);
        k += 12;
        len -= 12;
    }

    /*------------------------------------- handle the last 11 bytes */
    c += (uint32_t)length;
    switch(len) {            /* all the case statements fall through */
    case 11:
        c += ((ub4)k[10]<<24);
        /* Falls through. */
    case 10:
        c+=((ub4)k[9]<<16);
        /* Falls through. */
    case 9 :
        c+=((ub4)k[8]<<8);
        /* the first byte of c is reserved for the length */
        /* Falls through. */
    case 8 :
        b+=((ub4)k[7]<<24);
        /* Falls through. */
    case 7 :
        b+=((ub4)k[6]<<16);
        /* Falls through. */
    case 6 :
        b+=((ub4)k[5]<<8);
        /* Falls through. */
    case 5 :
        b+=k[4];
        /* Falls through. */
    case 4 :
        a+=((ub4)k[3]<<24);
        /* Falls through. */
    case 3 :
        a+=((ub4)k[2]<<16);
        /* Falls through. */
    case 2 :
        a+=((ub4)k[1]<<8);
        /* Falls through. */
    case 1 :
        a+=k[0];
        /* case 0: nothing left to add */
        /* Falls through. */
    }
    mix(a,b,c);
    /*-------------------------------------------- report the result */
    return c;
}

#ifdef _MSC_VER
#pragma warning(pop)
#endif


