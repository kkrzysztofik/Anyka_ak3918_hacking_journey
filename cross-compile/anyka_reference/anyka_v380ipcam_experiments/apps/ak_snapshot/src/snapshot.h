/*
 * snapshot.h
 *
 *  Created on: 19 de fev. de 2022
 *      Author: ricardo
 */

#ifndef SRC_SNAPSHOT_H_
#define SRC_SNAPSHOT_H_

typedef struct snapshot_t{

  int count;
  //pthread_cond_t ready; // bool
  uint8_t capture; // bool (indicates which BMP565 buffer bellow is used)
  int res_w;
  int res_h; //resolution
  unsigned short *rgb_565_n0; //snapshot
  unsigned short *rgb_565_n1; //snapshot

} snapshot_t;

#endif /* SRC_SNAPSHOT_H_ */
