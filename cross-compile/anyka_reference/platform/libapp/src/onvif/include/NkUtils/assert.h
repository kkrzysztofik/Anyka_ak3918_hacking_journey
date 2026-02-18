/**
 * Macro definitions used for project assertions.
 */

#include <NkUtils/types.h>
#include <NkUtils/log.h>
#include <NkUtils/stdos.h>
#include <string.h>

#ifndef NK_ASSERT_H_
#define NK_ASSERT_H_
NK_CPP_EXTERN_BEGIN

/**
 * Runtime code checkpoint.
 * Executing this macro outputs the file name and line number of that code location,
 * primarily used during debugging to locate where code execution has reached.
 */
#define NK_CHECK_POINT() \
	do {\
		NK_PChar file = (NK_PChar)(strrchr(__FILE__, '/'));\
		printf("\r\n--  @  %s:%d \r\n\r\n", !file ? __FILE__ : file + 1, __LINE__);\
	} while(0)

/**
 * Runtime code assertion.
 * When the expression logic is not true, program execution is terminated.
 * Typically used in unit tests to rigorously verify code behavior logic; assertions are disabled in release builds.
 */
#define NK_ASSERT(__exp) \
	do{\
		if(!(__exp)){\
			NK_Log()->alert("\"%s()\" Assertion Condition ( \"%s\" ) @ %d.", __PRETTY_FUNCTION__, #__exp, __LINE__);\
			exit(1);\
		}\
	}while(0)

/**
 * Assert condition is true.
 */
#define NK_ASSERT_TRUE(__condition) \
	NK_ASSERT(__condition)

/**
 * Assert condition is false.
 */
#define NK_ASSERT_FALSE(__condition) \
	NK_ASSERT(!(__condition))

/**
 * Conditional logic test.
 * Tests whether the logic in the condition is true; when not true, outputs a log based on the @ref __verbose flag.
 */
#define NK_TEST_(__condition, __verbose) \
	((__condition) ? NK_True : \
			(((__verbose) ? (NK_Log()->warn("\"%s() @ %d\" Expect Condition ( \"%s\" ).", __PRETTY_FUNCTION__, __LINE__, #__condition), NK_False) : 0), NK_False))

/**
 * Precondition expectation.
 * Outputs a hint log when the condition logic is not true.
 */
#define NK_EXPECT(__condition) \
	NK_TEST_(__condition, NK_True)

/**
 * Precondition expectation.
 * In a void function, exits the function when a precondition is not true, without any log output.
 */
#define NK_EXPECT_RETURN(__condition) \
	do{\
		if (!NK_TEST_(__condition, NK_False)){\
			return;\
		}\
	}while(0)

/**
 * Precondition expectation.
 * See @ref NK_EXPECT_RETURN(), for functions with a return value.
 */
#define NK_EXPECT_RETURN_VAL(__condition, __ret) \
	do{\
		if (!NK_TEST_(__condition, NK_False)){\
			return(__ret);\
		}\
	}while(0)


#define NK_EXPECT_JUMP(__condition, __location) \
	do{\
		if (!NK_TEST_(__condition, NK_False)){\
			goto __location;\
		}\
	}while(0)

/**
 * Precondition expectation.
 * See @ref NK_EXPECT_RETURN(), exits the function with a log output hint.
 */
#define NK_EXPECT_VERBOSE_RETURN(__condition) \
	do{\
		if (!NK_TEST_(__condition, NK_True)){\
			return;\
		}\
	}while(0)

/**
 * Precondition expectation.
 * See @ref NK_EXPECT_RETURN(), exits the function with a log output hint, for functions with a return value.
 */
#define NK_EXPECT_VERBOSE_RETURN_VAL(__condition, __ret) \
	do{\
		if (!NK_TEST_(__condition, NK_True)){\
			return(__ret);\
		}\
	}while(0)

#define NK_EXPECT_VERBOSE_JUMP(__condition, __location) \
	do{\
		if (!NK_TEST_(__condition, NK_True)){\
			goto __location;\
		}\
	}while(0)


NK_CPP_EXTERN_END
#endif /* NK_ASSERT_H_ */
