
//**************************
//*   print color start    *
//**************************
/*
Background color range: 40--49 Foreground color: 30--39

                40: Black                           30: Black
                41: Red                             31: Red
                42: Green                           32: Green
                43: Yellow                          33: Yellow
                44: Blue                            34: Blue
                45: Purple                          35: Purple
                46: Cyan                            36: Cyan
                47: White                           37: White

echo -e "\e[0;32;40m 0;32;40 \e[0m"
echo -e "\e[1;32;40m 1;32;40 \e[0m"
echo -e "\e[2;32;40m 2;32;40 \e[0m"
echo -e "\e[3;32;40m 3;32;40 \e[0m"
echo -e "\e[4;32;40m 4;32;40 \e[0m"
echo -e "\e[5;32;40m 5;32;40 \e[0m"
echo -e "\e[6;32;40m 6;32;40 \e[0m"
echo -e "\e[7;32;40m 7;32;40 \e[0m"
echo -e "\e[22;32;40m 22;32;40 \e[0m"
echo -e "\e[24;32;40m 24;32;40 \e[0m"
echo -e "\e[25;32;40m 25;32;40 \e[0m"
echo -e "\e[27;32;40m 27;32;40 \e[0m"
*/
#define COLOR_LEN                32
#define COLOR_BEGIN              "\033[%d;%d;%dm"
#define COLOR_END                "\033[0m"

#define COLOR_MODE_NORMAL        0                                                                                                          // Default mode
#define COLOR_MODE_BOLD          1                                                                                                          // Highlight
#define COLOR_MODE_UNDERLINED    4                                                                                                          // Underline
#define COLOR_MODE_BLINK         5                                                                                                          // Blink
#define COLOR_MODE_NEGATIVE      7                                                                                                          // Reverse color

#define COLOR_BACK_BLACK         40
#define COLOR_BACK_RED           41
#define COLOR_BACK_GREEN         42
#define COLOR_BACK_YELLOW        43
#define COLOR_BACK_BLUE          44
#define COLOR_BACK_PURPLE        45
#define COLOR_BACK_CYAN          46
#define COLOR_BACK_WHITE         47

#define COLOR_FRONT_BLACK        30
#define COLOR_FRONT_RED          31
#define COLOR_FRONT_GREEN        32
#define COLOR_FRONT_YELLOW       33
#define COLOR_FRONT_BLUE         34
#define COLOR_FRONT_PURPLE       35
#define COLOR_FRONT_CYAN         36
#define COLOR_FRONT_WHITE        37

//**************************
//*   print color end    *
//**************************
#define LEN_DEBUG                131072
#define LEN_TIME                 64
#define PROG_TRUE                1
#define PROG_FALSE               0
#define PROG_SUCCESS             0
#define PROG_FAILED             -1
#define EXIT_FAIL               -1
#define EXIT_NORMAL              0

typedef int pid_t ;
extern char gac_debug[ LEN_DEBUG ] , *gpc_prog_name , gc_run ;

char *get_color( int i_color_mode , int i_color_back , int i_color_front ) ;
pid_t gettid( ) ;
char *get_time_now( char *pc_time ) ;
void signal_exit( int i_sig ) ;


#define FREE_POINT( POINT )  \
if( POINT != NULL ) {\
    free( POINT ) ;\
    POINT = NULL ;\
}

//#define DEBUG_ANYKA                                                                                 // Set debug printing method
//#define DEBUG_PROG
#ifndef DEBUG_ANYKA
#ifndef DEBUG_PROG
#define DEBUG_PROG
#endif
#endif

#ifdef  DEBUG_PROG
#define DEBUG_SET_MAINPROG_NAME gpc_prog_name = argv[ 0 ] ;

#define DEBUG_THREAD_PRINT( MUTEX , COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... )  \
pthread_mutex_lock( &MUTEX ) ;\
snprintf( gac_debug , LEN_DEBUG , ##__VA_ARGS__ ) ;\
printf( "\033[%d;%d;%dm[%s] [%d:%d] %s:%s:%d:%s( ) %s%s\n" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , \
        get_time_now( NULL ) , getpid( ) , gettid( ) , gpc_prog_name , __FILE__ , __LINE__ , __func__ , gac_debug , \
        COLOR_END ) ;\
pthread_mutex_unlock( &MUTEX ) ;

#define DEBUG_PRINT( COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... )  \
snprintf( gac_debug , LEN_DEBUG , ##__VA_ARGS__ ) ;\
printf( "\033[%d;%d;%dm[%s] [%d:%d] %s:%s:%d:%s( ) %s%s\n" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , \
        get_time_now( NULL ) , getpid( ) , gettid( ) , gpc_prog_name , __FILE__ , __LINE__ , __func__ , gac_debug , \
        COLOR_END ) ;

#define DEBUG_HINT( COLOR_MODE , COLOR_BACK , COLOR_FRONT )  \
printf( "\033[%d;%d;%dm[%s] [%d:%d] %s:%s:%d:%s( )%s\n" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , \
        get_time_now( NULL ) , getpid( ) , gettid( ) , gpc_prog_name , __FILE__ , __LINE__ , __func__ , \
        COLOR_END ) ;

#define DEBUG_VAL( COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... )  \
snprintf( gac_debug , LEN_DEBUG , ##__VA_ARGS__ ) ;\
printf( "\033[%d;%d;%dm%s%s" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , gac_debug , COLOR_END ) ;
#endif

#ifdef  DEBUG_ANYKA
#define DEBUG_SET_MAINPROG_NAME gpc_prog_name = argv[ 0 ] ;

#define DEBUG_THREAD_PRINT( MUTEX , COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... )  \
pthread_mutex_lock( &MUTEX ) ;\
snprintf( gac_debug , LEN_DEBUG , ##__VA_ARGS__ ) ;\
printf( "\033[%d;%d;%dm[%s] [%d:%d] %s:%s:%d:%s( ) %s%s\n" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , \
        get_time_now( NULL ) , getpid( ) , gettid( ) , gpc_prog_name , __FILE__ , __LINE__ , __func__ , gac_debug , \
        COLOR_END ) ;\
pthread_mutex_unlock( &MUTEX ) ;

#define DEBUG_PRINT( COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... )  \
snprintf( gac_debug , LEN_DEBUG , ##__VA_ARGS__ ) ;\
printf( "\033[%d;%d;%dm[%s] [%d:%d] %s:%s:%d:%s( ) %s%s\n" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , \
        get_time_now( NULL ) , getpid( ) , gettid( ) , gpc_prog_name , __FILE__ , __LINE__ , __func__ , gac_debug , \
        COLOR_END ) ;

#define DEBUG_HINT( COLOR_MODE , COLOR_BACK , COLOR_FRONT )  \
printf( "\033[%d;%d;%dm[%s] [%d:%d] %s:%s:%d:%s( )%s\n" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , \
        get_time_now( NULL ) , getpid( ) , gettid( ) , gpc_prog_name , __FILE__ , __LINE__ , __func__ , \
        COLOR_END ) ;

#define DEBUG_VAL( COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... )  \
snprintf( gac_debug , LEN_DEBUG , ##__VA_ARGS__ ) ;\
printf( "\033[%d;%d;%dm%s%s" , \
        COLOR_MODE , COLOR_BACK , COLOR_FRONT , gac_debug , COLOR_END ) ;
#endif

#ifdef  DEBUG_NONE
#define DEBUG_SET_MAINPROG_NAME ;
#define DEBUG_THREAD_PRINT( MUTEX , COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... ) ;
#define DEBUG_PRINT( COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... ) ;
#define DEBUG_PRINTVAL( COLOR_MODE , COLOR_BACK , COLOR_FRONT , ... ) ;
#define DEBUG_HINT( COLOR_MODE , COLOR_BACK , COLOR_FRONT ) ;
#endif

struct tm *localtime_r(const time_t *timep, struct tm *result) ;
long syscall( long number , ... ) ;
