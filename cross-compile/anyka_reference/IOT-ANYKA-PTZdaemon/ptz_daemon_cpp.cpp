#define __cpp
#include "ak_extern.h"
#include <fcntl.h>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <sys/stat.h>
#include <thread>
#include <unistd.h>
#include <vector>

#define FIFOPATH "/tmp/ptz.daemon"

std::vector<std::string> operations = std::vector<std::string>();
int shuttingDown = 0;
int commandThreadDone = 0;
long long xpos = 180;
long long ypos = 90;
#define relativemove 10

/* stolen from
 * https://stackoverflow.com/questions/14265581/parse-split-a-string-in-c-using-string-delimiter-standard-c
 */
std::vector<std::string> splitString(const std::string &str, char delim) {
  std::vector<std::string> tokens;
  if (str == "")
    return tokens;
  std::string currentToken;
  std::stringstream ss(str);
  while (std::getline(ss, currentToken, delim)) {
    tokens.push_back(currentToken);
  }
  return tokens;
}

void relative_up(){
  std::cout << "turn up" << "\n";
  if (ypos >= relativemove){
    ypos -= relativemove;
  }
}
void relative_down(){
  std::cout << "turn down" << "\n";
  if (ypos <= 120-relativemove){
    ypos += relativemove;
  }
}
void relative_left(){
  std::cout << "turn left" << "\n";
  if (xpos >= relativemove){
    xpos -= relativemove;
  }
}
void relative_right(){
  std::cout << "turn right" << "\n";
  if (xpos <= 350-relativemove){
    xpos += relativemove;
  }
}

void process_command(const std::string l) {
  std::cout << "spawned processing command thread."
            << "\n";

  std::vector<std::string> cmd = splitString(l, ' ');
  std::cout << "parameters: \n";
  for (auto i : cmd) {
    std::cout << i << "\n";
  }

  //    __________________________________________________
  //   |                                                  |
  //   |                      GENERAL                     |
  //   |________________________--________________________|
  if (cmd[0] == "q") { //quit
      shuttingDown = 1;
  //    __________________________________________________
  //   |                                                  |
  //   |                 PTZ MOTOR MOTION                 |
  //   |________________________--________________________|
  } else if (cmd[0] == "init_ptz") {
      std::cout << "init ptz driver (anyka)" << "\n";
      ak_drv_ptz_open();
      ak_drv_ptz_check_self(0);
      ak_drv_ptz_set_degree(0x168, 0xc0); //360 horizontal, 192 vertical
      ak_drv_ptz_set_angle_rate(0x40000000, 0x40000000);
      xpos = 180;
      ypos = 90;
  // } else if (cmd[0] == "set_ar") {
  //     std::cout << "set angle rate" << "\n";
  //     ak_drv_ptz_set_angle_rate(atoll(cmd[1].c_str()), atoll(cmd[2].c_str()));
  // } else if (cmd[0] == "set_deg") {
  //     std::cout << "set degree" << "\n";
  //     ak_drv_ptz_set_degree(atoll(cmd[1].c_str()), atoll(cmd[2].c_str()));
  // } else if (cmd[0] == "set_speed") {
  //     std::cout << "set speed (motor, speed)" << "\n";
  //     ak_drv_ptz_set_speed(atoll(cmd[1].c_str()), atoll(cmd[2].c_str()), 0, 0);
  } else if (cmd[0] == "up") {
      relative_up();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "down") {
      relative_down();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "left") {
      relative_left();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "right") {
      relative_right();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "right_up") {
      relative_right();
      relative_up();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "right_down") {
      relative_right();
      relative_down();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "left_up") {
      relative_left();
      relative_up();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "left_down") {
      relative_left();
      relative_down();
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  } else if (cmd[0] == "t2p") {
      std::cout << "turn to position" << "\n";
      xpos = atoll(cmd[1].c_str());
      ypos = atoll(cmd[2].c_str());
      ak_drv_ptz_turn_to_pos(xpos, ypos);
  // } else if (cmd[0] == "t") {
  //     std::cout << "turn" << "\n";
  //     ak_drv_ptz_turn(atoll(cmd[1].c_str()), atoll(cmd[2].c_str()), 0);
  //    __________________________________________________
  //   |                                                  |
  //   |                        IR                        |
  //   |________________________--________________________|
  } else if (cmd[0] == "init_ir") {
      std::cout << "ir init" << "\n";
      ak_drv_ir_init();
  } else if (cmd[0] == "get_ir_in_lev") {
      int a = ak_drv_ir_get_input_level(0, 0, 0, 0);
      std::cout << "IR input level is: " << a << "\n";
  } else if (cmd[0] == "get_ir_thres") {
      int a = ak_drv_ir_get_threshold(0, 0);
      std::cout << "IR threshold is: " << a << "\n";
  } else if (cmd[0] == "set_ir_thres") {
      std::cout << "set IR threshold " << cmd[1] << "\n";
      ak_drv_ir_set_threshold((void *)cmd[1].c_str(), (void *)cmd[2].c_str());
  } else if (cmd[0] == "set_ir_cut") {
      std::cout << "set IR cut " << cmd[1] << "\n";
      ak_drv_ir_set_ircut(atoi(cmd[1].c_str()));
  } else if (cmd[0] == "set_ir_checkmode") {
      std::cout << "set IR checkmode" << "\n";
      ak_drv_ir_set_check_mode(atoi(cmd[1].c_str()));
  //    __________________________________________________
  //   |                                                  |
  //   |                      OTHER                       |
  //   |________________________--________________________|
  } else if (cmd[0] == "ping") {                         //ping
      std::cout << "pong" << "\n";
  }
  commandThreadDone = 1;
}

void thread_ready() {
  std::cout << "This thread is ready to be torn down!\n";
  commandThreadDone = 1;
}

void handle_fifo() {

  std::ifstream fifo;

  while (shuttingDown == 0) {
    fifo.open(FIFOPATH, std::ifstream::in);
    std::string l;
    while (std::getline(fifo, l)) {
      if (l.length() > 0)
        std::cout << "received something in the FIFO:" << l << "\n";
      else
        continue;

      std::cout << "Added operation to buffer\n";
      operations.push_back(l);
    }
    fifo.close();
    sleep(1);
  }
}

int main(int argc, char **argv) {

  struct stat buf;
  if (stat(FIFOPATH, &buf) != 0) {
    std::cout << "creating FIFO in /tmp..."
              << "\n";

    if ((mkfifo(FIFOPATH, 0700)) != 0) {
      std::cerr << "could not create fifo @ " << FIFOPATH << "\n";
      return 1;
    }
  }
  std::cout << "init ptz driver (anyka)\n";
  std::cout << "running as daemon now. awaiting commands in FIFO\n";

  shuttingDown = 0;
  std::thread t = std::thread(thread_ready);
  std::thread handleFIFO = std::thread(handle_fifo);
  while (shuttingDown == 0) {
    if (operations.size() > 0 && (!t.joinable() || commandThreadDone == 1)) {
      commandThreadDone = 0;
      std::cout << "thread is done - joining thread...\n";
      if (t.joinable())
        t.join();
      if (operations.size() > 0) {
        std::string line = operations.front();
        operations.erase(operations.begin());
        t = std::thread(process_command, line.c_str());
      }
    } else if (operations.size() > 0) {
      std::cout << "previous job is still running...\n";
    } else {
    }
    // std::cout << "there are currently " << operations.size()
    //           << " operations in queue.\n";

    sleep(1);
  }
  return 0;
}
