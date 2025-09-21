/*
 * snapshot.h
 *
 *  Created on: 19 de fev. de 2022
 *      Author: ricardo
 */

#ifndef SRC_SNAPSHOT_H_
#define SRC_SNAPSHOT_H_

typedef struct snapshot_t{ //this is the struct of information shared between the http and main encoder thread
  int count; //counts down how many snapshots to take (set by http thread, decreased by encoder)
  pthread_cond_t ready; // bool //signal for on-demand encoding
  uint8_t capture; // bool (indicates which image buffer bellow is being written to, the other can be served)
  int res_w; //resolution (from launch argument)
  int res_h;
  struct video_stream jpeg[2]; //2 snapshot buffers (contains .data and .len) [NOTE: length of JPEG can change unlike BMP which depends on resolution only]
} snapshot_t;

#endif /* SRC_SNAPSHOT_H_ */
