
/**
 * Log information control singleton module.
 */

#include <akae_typedef.h>

#if !defined(AKSPC_LOG_H_)
#define AKSPC_LOG_H_
AK_C_HEADER_EXTERN_C_BEGIN

/**
 * Maximum length of a single log output line.
 */
#define AK_LOG_1LINE_MAX_LEN (128)
#define AK_LOG_TAG_MAX_LEN   (8)

/**
 * Log output levels.
 */
#define AK_LOG_CLASS_ALLON   (0)
#define AK_LOG_CLASS_DEBUG2  (AK_LOG_CLASS_ALLON + 1)
#define AK_LOG_CLASS_DEBUG   (AK_LOG_CLASS_ALLON + 2)
#define AK_LOG_CLASS_INFO    (AK_LOG_CLASS_ALLON + 3)
#define AK_LOG_CLASS_WARN    (AK_LOG_CLASS_ALLON + 4)
#define AK_LOG_CLASS_ERROR   (AK_LOG_CLASS_ALLON + 5)
#define AK_LOG_CLASS_ALERT   (AK_LOG_CLASS_ALLON + 6)
#define AK_LOG_CLASS_ALLOFF  (AK_LOG_CLASS_ALLON + 7)

/**
 * Log output.
 * Through this interface, you can specify the color tag for the corresponding log output.
 */
AK_API AK_void akae_log_verbose(AK_int clss, AK_chrcptr tag, AK_chrcptr fmt, ...);


#define akae_log_tag_debug2(__tag, __fmt...)   akae_log_verbose(AK_LOG_CLASS_DEBUG2, (__tag), ##__fmt)
#define akae_log_tag_debug(__tag, __fmt...)    akae_log_verbose(AK_LOG_CLASS_DEBUG,  (__tag), ##__fmt)
#define akae_log_tag_info(__tag, __fmt...)     akae_log_verbose(AK_LOG_CLASS_INFO,   (__tag), ##__fmt)
#define akae_log_tag_warning(__tag, __fmt...)  akae_log_verbose(AK_LOG_CLASS_WARN,   (__tag), ##__fmt)
#define akae_log_tag_error(__tag, __fmt...)    akae_log_verbose(AK_LOG_CLASS_ERROR,  (__tag), ##__fmt)
#define akae_log_tag_alert(__tag, __fmt...)    akae_log_verbose(AK_LOG_CLASS_ALERT,  (__tag), ##__fmt)

#define akae_log_debug2(__fmt...)   akae_log_verbose(AK_LOG_CLASS_DEBUG2, AK_null, ##__fmt)
#define akae_log_debug(__fmt...)    akae_log_verbose(AK_LOG_CLASS_DEBUG,  AK_null, ##__fmt)
#define akae_log_info(__fmt...)     akae_log_verbose(AK_LOG_CLASS_INFO,   AK_null, ##__fmt)
#define akae_log_warning(__fmt...)  akae_log_verbose(AK_LOG_CLASS_WARN,   AK_null, ##__fmt)
#define akae_log_error(__fmt...)    akae_log_verbose(AK_LOG_CLASS_ERROR,  AK_null, ##__fmt)
#define akae_log_alert(__fmt...)    akae_log_verbose(AK_LOG_CLASS_ALERT,  AK_null, ##__fmt)


/**
 * Code runtime checkpoint.
 * Executing this macro in code outputs the file name and line number of that code location,
 * primarily used during debugging to locate the current code execution position.
 */
#define AK_CHECK_POINT() \
	do {\
		AK_chrptr file = akae_basename(__FILE__);\
		akae_log_debug("\r\n--  @  %s:%d \r\n", file, __LINE__);\
	} while(0)

/**
 * Code runtime assertion.
 * When the expression evaluates to false, the program terminates.
 * Typically used in unit testing for rigorous validation of code behavior;
 * assertions are disabled in release builds.
 */
#define AK_ASSERT(__exp) \
	do{\
		if(!(__exp)){\
			akae_log_alert("\"%s()\" Assertion Condition ( \"%s\" ) @ %d.", __PRETTY_FUNCTION__, #__exp, __LINE__);\
			exit(1);\
		}\
	}while(0)

/**
 * Assert that a condition is true.
 */
#define AK_ASSERT_TRUE(__condition) \
	AK_ASSERT(__condition)

/**
 * Assert that a condition is false.
 */
#define AK_ASSERT_FALSE(__condition) \
	AK_ASSERT(!(__condition))

/**
 * Conditional logic test.
 * Tests whether the condition is true; when false, outputs a log according to the @ref __verbose flag.
 */
#define AK_TEST(__condition, __verbose) \
	((__condition) ? AK_true : \
		((__verbose) ? \
			(akae_log_warning("\"%s:%d\" Condition ( \"%s\" ) Failed.", akae_basename (__FILE__), __LINE__, #__condition), AK_false) : AK_false))

/**
 * Conditional expectation.
 * Outputs a warning log when the condition is not true.
 */
#define AK_EXPECT(__condition) \
	AK_TEST(__condition, AK_true)

/**
 * Conditional expectation.
 * In a method, evaluates a precondition; for void methods, exits the method silently when the condition is false.
 */
#define AK_EXPECT_RETURN(__condition) \
	do{\
		if (!AK_TEST(__condition, AK_false)){\
			return;\
		}\
	}while(0)

/**
 * Conditional expectation.
 * See @ref AK_EXPECT_RETURN(), for methods with a return value.
 */
#define AK_EXPECT_RETURN_VAL(__condition, __ret) \
	do{\
		if (!AK_TEST(__condition, AK_false)){\
			return(__ret);\
		}\
	}while(0)

/**
 * Conditional expectation.
 * When the condition is not met, jumps to the @ref __location code label.
 */
#define AK_EXPECT_JUMP(__condition, __location) \
	do{\
		if (!AK_TEST(__condition, AK_false)){\
			goto __location;\
		}\
	}while(0)

/**
 * Conditional expectation.
 * When the condition is not met, breaks out of the current loop.
 */
#define AK_EXPECT_BREAK(__condition) \
	if (!AK_TEST(__condition, AK_false)){\
		break;\
	}\

/**
 * Conditional expectation.
 * When the condition is not met, continues to the next iteration of the current loop.
 */
#define AK_EXPECT_CONTINUE(__condition) \
	if (!AK_TEST(__condition, AK_false)){\
		continue;\
	}\


/**
 * Conditional expectation.
 * Like @ref AK_EXPECT_RETURN(), but outputs a log message when exiting.
 */
#define AK_EXPECT_VERBOSE_RETURN(__condition) \
	do{\
		if (!AK_TEST(__condition, AK_true)){\
			return;\
		}\
	}while(0)

/**
 * Conditional expectation.
 * Like @ref AK_EXPECT_RETURN(), but outputs a log message when exiting, for methods with a return value.
 */
#define AK_EXPECT_VERBOSE_RETURN_VAL(__condition, __ret) \
	do{\
		if (!AK_TEST(__condition, AK_true)){\
			return(__ret);\
		}\
	}while(0)

#define AK_EXPECT_VERBOSE_JUMP(__condition, __location) \
	do{\
		if (!AK_TEST(__condition, AK_true)){\
			goto __location;\
		}\
	}while(0)


#define AK_EXPECT_VERBOSE_BREAK(__condition) \
	if (!AK_TEST(__condition, AK_true)){\
		break;\
	}\


#define AK_EXPECT_VERBOSE_CONTINUE(__condition) \
	if (!AK_TEST(__condition, AK_true)){\
		continue;\
	}\



AK_C_HEADER_EXTERN_C_END
#endif ///< LOG_H_
