
/**
 * Log information control singleton module.
 */

#include <akae_typedef.h>
#include <akae_log.h>

#if !defined(AKSPC_STDLIB_H_)
#define AKSPC_STDLIB_H_
AK_C_HEADER_EXTERN_C_BEGIN



/**
 * Detect whether the current platform is little-endian.
 *
 * @return
 *  Returns AK_true if the current platform is little-endian,\n
 *  returns AK_false if big-endian.
 */
AK_API AK_boolean akae_little_endian(void);


/**
 * Get the compilation date and time of the current program.\n
 * Internally obtained by parsing the compiler's __DATE__ and __TIME__ macros.\n
 * Pass AK_null for any variable you do not need.
 *
 * @param[out]
 *  year
 *  Program compilation year (>1900).
 * @param[out]
 *  month
 *  Program compilation month (1 - 12).
 * @param[out]
 *  mday
 *  Program compilation day (1 - 31).
 * @param[out]
 *  hour
 *  Program compilation hour (1 - 23).
 * @param[out]
 *  min
 *  Program compilation minute (1 - 59).
 * @param[out]
 *  sec
 *  Program compilation second (1 - 59).
 *
 */
AK_API AK_void akae_whenbuild(AK_int *year, AK_int *month,
		AK_int *mday, AK_int *hour, AK_int *min, AK_int *sec);


/**
 * Compare two memory regions to check whether their data matches.\n
 * The caller must ensure the validity of the memory lengths when calling this interface.
 *
 * @param[in]
 *  s1
 *  First memory region.
 * @param[in]
 *  s2
 *  Second memory region.
 * @param[in]
 *  n
 *  Length of memory to compare (in bytes).
 *
 * @return
 *  Returns 0 if @ref n is 0.\n
 *  Returns 0 if the @ref n bytes of both memory regions match,\n
 *  returns a value less than 0 if @ref s1 data is less than @ref s2 data within @ref n bytes,\n
 *  returns a value greater than 0 if greater.
 *
 */
AK_API AK_int akae_memcmp(AK_bytptr s1, AK_bytptr s2, AK_size n);



AK_API AK_voidptr akae_memchr(AK_voidcptr s, AK_char chr, AK_size n);


AK_API AK_voidptr akae_memset(AK_voidptr s, AK_char c, AK_size n);

/**
 * Clear memory data to 0; commonly used for memory initialization.
 */
AK_API AK_voidptr akae_memzero(AK_voidptr s, AK_size n);

/**
 * Quick call to zero-initialize an array.
 */
#define AK_MEMZERO_ARRAY(__array) \
		akae_memzero(__array, sizeof(__array));

/**
 * Quick call to zero-initialize an object (struct or union).
 */
#define AK_MEMZERO_OBJECT(__object) \
		akae_memzero(&(__object), sizeof(__object));


/*
 * Find the first occurrence of the byte string needle in byte string haystack.
 */
AK_API AK_voidptr akae_memmem (AK_voidcptr haystack, AK_size haystacklen,
		AK_voidcptr needle, AK_size needlelen);

AK_API AK_size akae_strlen(AK_chrcptr s);

AK_API AK_chrptr akae_strstr(AK_chrcptr haystack, AK_chrptr needle);


AK_API AK_chrptr akae_strcpy(AK_chrptr dest, AK_chrcptr src);

AK_API AK_chrptr akae_strncpy(AK_chrptr dest, AK_chrcptr src, AK_size n);

/**
 * Trim a string.\n
 * Removes consecutive spaces from the beginning and end of the string.\n
 *
 * @param[in]
 *  text
 *  Input string.
 * @param[out]
 *  stack
 *  Stack memory buffer for storing the output result string.\n
 *  Length is specified by @ref stacklen.
 * @param[in]
 *  stacklen
 *  Stack memory length.
 *
 * @return
 *  Output string.
 */
AK_API AK_chrptr akae_strtrim(AK_chrcptr text,
		AK_chrptr stack, AK_size stacklen);

/**
 * String background fill.\n
 * Pads the user-provided content with a background character and outputs it at the specified length.\n
 * For example, text can be output as --text-- by filling with the - character.
 *
 * @param[in]
 *  text
 *  Input string.
 * @param[in]
 *  bgchr
 *  Background character; must be a printable character.
 * @param[out]
 *  stack
 *  Stack memory buffer for storing the output result string.\n
 *  Length is specified by @ref stacklen.
 * @param[in]
 *  stacklen
 *  Stack memory length; the output string length is determined by @ref stacklen.\n
 *  String length equals @ref stacklen minus 1.
 *
 * @return
 *  Output string.
 */
AK_API AK_chrptr akae_strbgchr(AK_chrcptr text, AK_char bgchr,
		AK_chrptr stack, AK_size stacklen);

/**
 * String comparison.\n
 * Returns True when the two strings are identical, otherwise returns False.
 */
AK_API AK_boolean akae_streq(AK_chrcptr str1, AK_chrcptr str2);

/**
 * Case-insensitive comparison; see @ref akae_streq().
 */
AK_API AK_boolean akae_strcaseeq(AK_chrcptr str1, AK_chrcptr str2);

/**
 * String prefix comparison.
 * Returns True when the two strings share the same prefix,\n
 * for example, "absolute" matches prefix "abs".\n
 * Otherwise returns False.
 */
AK_API AK_boolean akae_strprefixeq(AK_chrcptr str, AK_chrcptr prefix);

/**
 * Case-insensitive comparison; see @ref akae_strprefixeq().
 */
AK_API AK_boolean akae_strprefixcaseeq(AK_chrcptr str, AK_chrcptr prefix);

/**
 * String suffix comparison.\n
 * Returns True when the two strings share the same suffix, otherwise returns False.
 */
AK_API AK_boolean akae_strsuffixeq(AK_chrcptr str, AK_chrcptr suffix);

/**
 * Case-insensitive comparison; see @ref akae_strsuffixeq().
 */
AK_API AK_boolean akae_strsuffixcaseeq(AK_chrcptr str, AK_chrcptr suffix);


/**
 * Convert all characters in a string to uppercase.
 */
AK_API AK_chrptr akae_strupper(AK_chrcptr text,
		AK_chrptr stack, AK_size stacklen);

/**
 * Convert all characters in a string to lowercase.
 */
AK_API AK_chrptr akae_strlower(AK_chrcptr text,
		AK_chrptr stack, AK_size stacklen);

/**
 * Memory copy.\n
 * The two memory regions may overlap; the function automatically detects and handles this.
 *
 * @return
 *  Returns the starting address of the destination memory.
 */
AK_API AK_bytptr akae_memcpy(AK_voidptr dest, AK_voidcptr src, AK_size n);


/**
 * Inspect memory data and print it to the terminal in hexadecimal format.
 *
 * @param[in]
 *  mem
 * @param[in]
 *  len
 */
AK_API AK_void akae_hexdump(AK_voidptr mem, AK_size len);


/**
 * @brief
 *  Get the process start timestamp (unit: nanoseconds).
 */
AK_API AK_size64 akae_clock_nano(void);

/**
* @brief
*  Get the system timestamp (unit: microseconds).
*/
AK_API AK_size64 akae_clock_macro(void);

/**
 * Measure the execution time of a statement (unit: microseconds).
 */
#define AK_TIME(__process) \
	({\
		AK_size64 us = akae_clock_macro ();\
		__process;\
		akae_clock_macro () - us;\
	})

/**
* @brief
*  Get the system timestamp (unit: milliseconds).
*/
AK_API AK_size64 akae_clock_milli(void);

/**
* @brief
*  Get the system timestamp (unit: seconds).
*/
AK_API AK_size64 akae_clock(void);


/**
 * @brief
 *  Output the trailing path component with directory stripped.
 */
AK_API AK_chrptr akae_basename(AK_chrptr path);

AK_API AK_boolean akae_has_file(AK_chrptr path);

/**
 * @brief
 *  Return the number of seconds since Epoch, 1970-01-01 00:00:00 +0000.
 */
AK_API AK_uint32 akae_utc();

AK_API AK_void akae_sleep(AK_size sec);

AK_API AK_void akae_usleep(AK_size usec);

AK_API AK_int akae_atoi(AK_chrptr str);

AK_API AK_int64 akae_atoi64(AK_chrptr str);




/**
 * Little-endian alignment correction for data.
 */
#define AK_ALIGN_LEFT_END(__v, __av) \
	((__av) <= 0 ? 0 : (\
			((__v > 0) ?\
					((__v) - ((__v) % (__av)))\
					:\
					 ((__v) - ((__v) % (__av)) - ((0 != ((__v) % (__av))) ? (__av) : 0))\
					)))

/**
 * Big-endian alignment correction for data.
 */
#define AK_ALIGN_RIGHT_END(__v, __av) \
		((__av) <= 0 ? 0 : (\
			((__v > 0) ?\
					((__v) - ((__v) % (__av)) + ((0 != ((__v) % (__av))) ? (__av) : 0))\
					:\
					 ((__v) - ((__v) % (__av)))\
					)))


/**
 * 16-bit byte-order swap.
 */
#define AK_SWAP16(__v)  ((((AK_uint16)(__v) & 0xff00) >> 8) | (((AK_uint16)(__v) & 0x00ff) << 8))
 // Long integer byte-order swap

/**
 * 32-bit byte-order swap.
 */
#define AK_SWAP32(__v)  ((((AK_uint32)(__v) & 0xff000000) >> 24) |\
                            (((AK_uint32)(__v) & 0x00ff0000) >> 8) |\
                            (((AK_uint32)(__v) & 0x0000ff00) << 8) |\
                            (((AK_uint32)(__v) & 0x000000ff) << 24))


/**
 * Convert 4-byte data from host byte order to network byte order.
 */
#define AK_HTON32(__h) \
	(akae_little_endian() ? AK_SWAP32(__h) : __h)

/**
 * Convert 3-byte data from host byte order to network byte order.
 */
#define AK_HTON24(__h) (AK_HTON32(__h) >> 8)

/**
 * Convert 2-byte data from host byte order to network byte order.
 */
#define AK_HTON16(__h) \
	(akae_little_endian() ? AK_SWAP16(__h) : __h)


/**
 * Convert 4-byte data from network byte order to host byte order.
 */
#define AK_NTOH32(__n) AK_HTON32(__n)

/**
 * Convert 3-byte data from network byte order to host byte order.
 */
#define AK_NTOH24(__n) (AK_NTOH32(__n) >> 8)

/**
 * Convert 2-byte data from network byte order to host byte order.
 */
#define AK_NTOH16(__n) AK_HTON16(__n)


/// @brief Return the minimum of two values.
#define AK_MIN(__a, __b) ((__a) <= (__b) ? (__a) : (__b))

/// @brief Return the maximum of two values.
#define AK_MAX(__a, __b) ((__a) >= (__b) ? (__a) : (__b))

/**
 * @macro
 *  Clamp variable @ref __v within the specified min/max range.
 */
#define AK_FIX_RANGE(__v, __min, __max) \
	(__min) <= (__max) ? AK_MAX(AK_MIN(__v, (__max)), (__min)) : AK_MAX(AK_MIN(__v, (__min)), (__max))



AK_C_HEADER_EXTERN_C_END
#endif ///< STDLIB_H_
