#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>
#include <fcntl.h>
#include <signal.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <pthread.h> //for threading , link with lpthread
#include "snapshot.h"
#include "log.h"
#include "convert.h"

// Source adapted from: https://github.com/ankushagarwal/nweb

#define VERSION 23
#define BUFSIZE 65536
#define ERROR      42
#define LOG        44
#define FORBIDDEN 403
#define NOTFOUND  404

pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;

int server_fd;
char bmp_data_path[50];

struct {
  char *ext;
  char *filetype;
} extensions [] = {
  //{"gif", "image/gif" },
  {"bmp", "image/bmp" },
  //{"jpg", "image/jpg" },
  //{"jpeg","image/jpeg"},
  //{"png", "image/png" },
  {"ico", "image/ico" },
  //{"zip", "image/zip" },
  //{"gz",  "image/gz"  },
  //{"tar", "image/tar" },
  //{"htm", "text/html" },
  //{"html","text/html" },
  {0,0} };

struct snapshot_t *snapshot_http = NULL;

void logger(int type, char *s1, char *s2, int socket_fd)
{
  int fd ;

  switch (type) {
  case ERROR: (void)printf("ERROR: %s:%s Errno=%d exiting pid=%d",s1, s2, errno,getpid());
    break;
  case FORBIDDEN:
    //(void)write(socket_fd, "HTTP/1.1 403 Forbidden\nContent-Length: 185\nConnection: close\nContent-Type: text/html\n\n<html><head>\n<title>403 Forbidden</title>\n</head><body>\n<h1>Forbidden</h1>\nThe requested URL, file type or operation is not allowed on this simple static file webserver.\n</body></html>\n",272);
    (void)write(socket_fd, "HTTP/1.1 403 Forbidden",23);
    (void)printf("FORBIDDEN: %s:%s",s1, s2);
    break;
  case NOTFOUND:
    //(void)write(socket_fd, "HTTP/1.1 404 Not Found\nContent-Length: 144\nConnection: close\nContent-Type: text/html\n<html><head>\n<title>404 Not Found</title>\n</head><body>\n<h1>Not Found</h1>\nThe requested URL was not found on this server.\n</body></html>\n",225);
    (void)write(socket_fd, "HTTP/1.1 404 Not Found",23);
    (void)printf("NOT FOUND: %s:%s",s1, s2);
    break;
  case LOG: (void)printf("INFO: %s:%s:%d\n",s1, s2,socket_fd); break;
  }

}

/* this is a child web server process, so we can exit on errors */
void web(int fd, int hit)
{
  int j, file_fd, buflen;
  long i, ret, len;
  char * fstr;
  static char buffer[BUFSIZE+1]; /* static so zero filled */


  logi("new request !!!");

  ret =read(fd,buffer,BUFSIZE);   /* read Web request in one go */
  if(ret == 0 || ret == -1) {  /* read failure stop now */
    logger(FORBIDDEN,"failed to read browser request","",fd);
  }
  if(ret > 0 && ret < BUFSIZE)  /* return code is valid chars */
    buffer[ret]=0;    /* terminate the buffer */
  else buffer[0]=0;
  for(i=0;i<ret;i++)  /* remove CF and LF characters */
    if(buffer[i] == '\r' || buffer[i] == '\n')
      buffer[i]='*';
  logger(LOG,"request",buffer,hit);
  if( strncmp(buffer,"GET ",4) && strncmp(buffer,"get ",4) ) {
    logger(FORBIDDEN,"Only simple GET operation supported",buffer,fd);
  }
  for(i=4;i<BUFSIZE;i++) { /* null terminate after the second space to ignore extra stuff */
    if(buffer[i] == ' ') { /* string is "GET URL " +lots of other stuff */
      buffer[i] = 0;
      break;
    }
  }
  for(j=0;j<i-1;j++)   /* check for illegal parent directory use .. */
    if(buffer[j] == '.' && buffer[j+1] == '.') {
      logger(FORBIDDEN,"Parent directory (..) path names not supported",buffer,fd);
    }
  if( !strncmp(&buffer[0],"GET /\0",6) || !strncmp(&buffer[0],"get /\0",6) ) /* convert no filename to index file */
    (void)strcpy(buffer,"GET /index.html");

  /* work out the file type and check we support it */
  buflen=strlen(buffer);
  fstr = (char *)0;
  for(i=0;extensions[i].ext != 0;i++) {
    len = strlen(extensions[i].ext);
    if( !strncmp(&buffer[buflen-len], extensions[i].ext, len)) {
      fstr = extensions[i].filetype;
      break;
    }
  }
  if(fstr == 0) logger(FORBIDDEN,"file extension type not supported",buffer,fd);


  // SNAPSHOT
  // Example: http://192.168.15.57:3000/any.bmp
  if(!strcmp(fstr, "image/bmp")){

	//snapshot_http->ready = 0;
	//pthread_cond_init(&snapshot_http->ready, NULL);
	//snapshot_http->capture = 1;

	//pthread_mutex_lock(&lock);
	//logw("Wait capture...");
	//pthread_cond_wait(&snapshot_http->ready, &lock);
	//pthread_mutex_unlock(&lock);

  BITMAPFILEHEADER BmpFileHeader;
  BITMAPINFOHEADER BmpInfoHeader;
  unsigned int rgb[3];

  /* Fill header information */
  BmpFileHeader.bfType = 0x4d42;
  BmpFileHeader.bfSize = snapshot_http->res_w*snapshot_http->res_h*2 + sizeof(BmpFileHeader) + sizeof(BmpInfoHeader) + sizeof(rgb);
  BmpFileHeader.bfReserved1 = 0;
  BmpFileHeader.bfReserved2 = 0;
  BmpFileHeader.bfOffBits = sizeof(BmpFileHeader) + sizeof(BmpInfoHeader) + sizeof(rgb);

  BmpInfoHeader.biSize = sizeof(BmpInfoHeader);
  BmpInfoHeader.biWidth = snapshot_http->res_w;
  BmpInfoHeader.biHeight = -snapshot_http->res_h;
  BmpInfoHeader.biPlanes = 0x01;
  BmpInfoHeader.biBitCount = 16;
  BmpInfoHeader.biCompression = 3;
  BmpInfoHeader.biSizeImage = (BmpFileHeader.bfSize + 3) & ~3;
  //BmpInfoHeader.biXPelsPerMeter = 0;
  //BmpInfoHeader.biYPelsPerMeter = 0;
  BmpInfoHeader.biClrUsed = 0;
  BmpInfoHeader.biClrImportant = 0;

  rgb[0] = 0xf800;
  rgb[1] = 0x07e0;
  rgb[2] = 0x001f;

  logger(LOG,"SEND",&buffer[5],hit);

  //len = (long)lseek(file_fd, (off_t)0, SEEK_END); /* lseek to the file end to find the length */
  //      (void)lseek(file_fd, (off_t)0, SEEK_SET); /* lseek back to the file start ready for reading */
  len = snapshot_http->res_w*snapshot_http->res_h*2 + sizeof(BmpFileHeader) + sizeof(BmpInfoHeader) + sizeof(rgb);
          (void)sprintf(buffer,"HTTP/1.1 200 OK\nServer: nweb/%d.0\nContent-Length: %ld\nConnection: close\nContent-Type: %s\n\n", VERSION, len, fstr); /* Header + a blank line */

          logger(LOG,"Header",buffer,hit);
  (void)write(fd,buffer,strlen(buffer));

  //fwrite(&BmpFileHeader,sizeof(BmpFileHeader),1,fp);
  (void)write(fd,&BmpFileHeader,sizeof(BmpFileHeader));
  //fwrite(&BmpInfoHeader,sizeof(BmpInfoHeader),1,fp);
  (void)write(fd,&BmpInfoHeader,sizeof(BmpInfoHeader));
  //fwrite(rgb,sizeof(rgb),1,fp);
  (void)write(fd,rgb,sizeof(rgb));
  //fwrite(rgb_565,width*height*2,1,fp);
  //(void)printf("Serving from RGB565 buffer: %d\n",!snapshot_http->capture);
  if (snapshot_http->capture == 0){
    (void)write(fd,snapshot_http->rgb_565_n1,snapshot_http->res_w*snapshot_http->res_h*2);
  }else{
    (void)write(fd,snapshot_http->rgb_565_n0,snapshot_http->res_w*snapshot_http->res_h*2);
  }
  //free(snapshot_http->rgb_565);
  //snapshot_http->rgb_565 = NULL;

  /* send file in blocks - last block may be smaller */
  //while (  (ret = read(file_fd, buffer, BUFSIZE)) > 0 ) {
  //  (void)write(fd,buffer,ret);
  //}

	// TODO: IMPLEMENT WRITE/READ IN MEMORY !
	//if(( file_fd = open(bmp_data_path,O_RDONLY)) == -1) {  /* open the file for reading */
	//	logger(NOTFOUND, "failed to open file",&buffer[5],fd);
	//}

  // Serve other files
  }else{
	//snapshot_http->capture = 1;
	 //if(( file_fd = open(&buffer[5],O_RDONLY)) == -1) {  /* open the file for reading */
		 logger(NOTFOUND, "failed to open file",&buffer[5],fd);
	 //}
  }


  sleep(0.01);  /* allow socket to drain before signalling the socket is closed */
  close(fd);
}

/*
 * This will handle connection for each client
 * */
void *accept_thread(void *server_fd)
{
	int i, client_fd, hit;
	static struct sockaddr_in cli_addr; /* static = initialised to zeros */
	socklen_t length;

	for (hit = 1;; hit++) {
		length = sizeof(cli_addr);
		logi("Waiting for client...");
		if ((client_fd = accept((int *)server_fd, (struct sockaddr*) &cli_addr, &length))< 0)
			loge("error accept....");
		else {
			logv("Accepted !!");
			web(client_fd, hit);
			client_fd = 0;
                        //break;
		}
	}

	logi("Server loop [ENDs]");
}

int start_server(int port, char *webdir, snapshot_t *snapshot){
	int i, pid;

	static struct sockaddr_in serv_addr; /* static = initialised to zeros */
	snapshot_http = snapshot;

	logi("Webserver starting: %d, pid %d \n", port , getpid());

	/* setup the network socket */
	if ((server_fd = socket(AF_INET, SOCK_STREAM, 0)) < 0)
		loge("Could not create server socket");


	serv_addr.sin_family = AF_INET;
	serv_addr.sin_addr.s_addr = htonl(INADDR_ANY);
	serv_addr.sin_port = htons(port);
        //add webdir parameter (gerge)
	sprintf(bmp_data_path, "%s/preview.bmp", webdir);
	if (bind(server_fd, (struct sockaddr*) &serv_addr, sizeof(serv_addr)) < 0)
		loge("fail socket bind");

	if (listen(server_fd, 64) < 0)
		loge("fail socket listen");

	pthread_t thread_id;
	if( pthread_create( &thread_id , NULL , accept_thread, (void*) server_fd) < 0)
	{
		perror("could not create thread");
		return -1;
	}

	return 0;
}


void stop_server(){

	close(server_fd);

}
