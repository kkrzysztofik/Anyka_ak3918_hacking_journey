/**
 * Basic operation macro definitions.
 */

#include <NkUtils/types.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <NkUtils/stdos.h>


#ifndef NK_MACRO_H_
#define NK_MACRO_H_
NK_CPP_EXTERN_BEGIN



/**
 * Little-endian data alignment correction.
 */
#define NK_ALIGN_LITTLE_END(__v, __av) \
	((__av) <= 0 ? 0 : (\
			((__v > 0) ?\
					((__v) - ((__v) % (__av)))\
					:\
					 ((__v) - ((__v) % (__av)) - ((0 != ((__v) % (__av))) ? (__av) : 0))\
					)))

/**
 * Big-endian data alignment correction.
 */
#define NK_ALIGN_BIG_END(__v, __av) \
		((__av) <= 0 ? 0 : (\
			((__v > 0) ?\
					((__v) - ((__v) % (__av)) + ((0 != ((__v) % (__av))) ? (__av) : 0))\
					:\
					 ((__v) - ((__v) % (__av)))\
					)))


/**
 * 16-bit endian swap.
 */
#define NK_SWAP16(__v)  ((((NK_UInt16)(__v) & 0xff00) >> 8) | (((NK_UInt16)(__v) & 0x00ff) << 8))
 // Long integer endian swap.

/**
 * 32-bit endian swap.
 */
#define NK_SWAP32(__v)  ((((NK_UInt32)(__v) & 0xff000000) >> 24) |\
                            (((NK_UInt32)(__v) & 0x00ff0000) >> 8) |\
                            (((NK_UInt32)(__v) & 0x0000ff00) << 8) |\
                            (((NK_UInt32)(__v) & 0x000000ff) << 24))


/// Clear memory block.
#define NK_BZERO(__ptr, __size) do { memset(__ptr, 0, __size); } while(0)

/**
 * @macro
 *  String comparison.\n
 *  Returns True when the two strings are identical, otherwise returns False.
 */
#define NK_STREQ(__str1, __str2) \
	(((strlen(__str1) == strlen(__str2)) && (0 == strcmp(__str1, __str2))) ? NK_True : NK_False)

/// Case-insensitive string matching.
/// Returns True when the two strings are identical ignoring case, otherwise returns False.
#define NK_STRCASEEQ(__str1, __str2) \
	(((strlen(__str1) == strlen(__str2)) && (0 == strcasecmp(__str1, __str2))) ? NK_True : NK_False)

/// String prefix comparison.
/// Returns True when the two strings share the same prefix, otherwise returns False.
#define NK_PREFIX_STREQ(__str, __pre) \
	(strlen(__str) < strlen(__pre) ? NK_False : (0 == strncmp(__str, __pre, strlen(__pre))))

/// Case-insensitive string prefix comparison.
/// Returns True when the two strings share the same prefix ignoring case, otherwise returns False.
#define NK_PREFIX_STRCASEEQ(__str, __pre) \
	(strlen(__str) < strlen(__pre) ? NK_False : (0 == strncasecmp(__str, __pre, strlen(__pre))))

/// String suffix comparison.
/// Returns True when the two strings share the same suffix, otherwise returns False.
#define NK_SUFFIX_STREQ(__str, __sff) \
	(strlen(__str) < strlen(__sff) ? NK_False : (0 == strcmp(__str + strlen(__str) - strlen(__sff), __sff)))

/// Case-insensitive string suffix comparison.
/// Returns True when the two strings share the same suffix ignoring case, otherwise returns False.
#define NK_SUFFIX_STRCASEEQ(__str, __sff) \
	(strlen(__str) < strlen(__sff) ? NK_False : (0 == strcasecmp(__str + strlen(__str) - strlen(__sff), __sff)))

/**
 * @macro
 *  Alternative naming style for the NK_*_STR*EQ() series of interfaces.
 */
#define NK_STRCMP 				NK_STREQ
#define NK_STRCASECMP			NK_STRCASEEQ
#define NK_PREFIX_STRCMP		NK_PREFIX_STREQ
#define NK_PREFIX_STRCASECMP	NK_PREFIX_STRCASEEQ
#define NK_SUFFIX_STRCMP		NK_SUFFIX_STREQ
#define NK_SUFFIX_STRCASECMP	NK_SUFFIX_STRCASEEQ

/**
 * String trimming.\n
 * Removes consecutive spaces at the beginning and end of the string.\n
 *
 * @param[in]			text			Input string.
 * @param[in,out]		stack			Output string.
 * @param[in]			stack_len		Length of the stack buffer.
 *
 * @return		Output string.
 */
static inline NK_PChar
NK_STR_TRIM(NK_PChar text, NK_PChar stack, NK_Size stack_len)
{
	NK_Size text_len = strlen(text);
	NK_PChar end_pos = NK_Nil;
	NK_PChar start_pos = NK_Nil;

	if (!stack || stack_len < text_len) {
		/// Return buffer error.
		return NK_Nil;
	}

	/// Find the start position of non-space characters.
	text_len = strlen(text);
	for (start_pos = text; start_pos < text + text_len + 1; ++start_pos) {
		if (' ' != start_pos[0]) {
			text_len = snprintf(stack, stack_len, "%s", start_pos);
			break;
		}
	}

	/// First clear trailing consecutive spaces from the string.
	for (end_pos = stack + text_len - 1; end_pos >= stack && ' ' == end_pos[0]; --end_pos) {
		end_pos[0] = '\0';
	}

	/// Return the trimmed string.
	return stack;
}

/**
 * Convert all characters in the string to uppercase.
 */
static inline NK_Void
NK_STR_TOUPPER(NK_PChar text)
{
	NK_Size len = strlen(text);
	NK_Int i = 0;
	for (i = 0; i < (int)len; ++i) {
		NK_Char chr = text[i];
		if (chr >= 'a' && chr <= 'z') {
			text[i] = chr - 'a' + 'A';
		}
	}
}

/**
 * Convert all characters in the string to lowercase.
 */
static inline NK_Void
NK_STR_TOLOWER(NK_PChar text)
{
	NK_Size len = strlen(text);
	NK_Int i = 0;
	for (i = 0; i < (int)len; ++i) {
		NK_Char chr = text[i];
		if (chr >= 'A' && chr <= 'Z') {
			text[i] = chr - 'A' + 'a';
		}
	}
}

/**
 * Detect whether the system is little-endian.
 * Returns True for little-endian, False for big-endian.
 */
static inline NK_Boolean NK_IS_LITTLE_END()
{
	union{
    	   NK_UInt32 dword;
    	   NK_UInt8 bytes[4];
	} un;

	un.dword = 0x12345678;
	return (0x78 == un.bytes[0]);
}

/**
 * Convert 4-byte data from host byte order to network byte order.
 */
static inline NK_UInt32 NK_HTON32(NK_UInt32 h)
{
	/// If host is big-endian, same as network order, return directly.
	/// If host is little-endian, convert to big-endian before returning.
	return NK_IS_LITTLE_END() ? NK_SWAP32(h) : h;
}

#define NK_SELF_HTON32(__h) do { (__h) = NK_HTON32(__h); } while(0)

/**
 * Convert 3-byte data from host to network byte order.
 */
#define NK_HTON24(__h) (NK_HTON32(__h) >> 8)

/**
 * Convert 2-byte data from host byte order to network byte order.
 */
static inline NK_UInt16 NK_HTON16(NK_UInt16 h)
{
	/// If host is big-endian, same as network order, return directly.
	/// If host is little-endian, convert to big-endian before returning.
	return NK_IS_LITTLE_END() ? NK_SWAP16(h) : h;
}

#define NK_SELF_HTON16(__h) do { (__h) = NK_HTON16(__h); } while(0)

/**
 * Convert 4-byte data from network byte order to host byte order.
 */
#define NK_NTOH32(__n) NK_HTON32(__n)
#define NK_SELF_NTOH32(__n) do { (__n) = NK_NTOH32(__n); } while(0)

/**
 * Convert 3-byte data from network to host byte order.
 */
#define NK_NTOH24(__n) (NK_NTOH32(__n) >> 8)

/**
 * Convert 2-byte data from network byte order to host byte order.
 */
#define NK_NTOH16(__n) NK_HTON16(__n)
#define NK_SELF_NTOH16(__n) do { (__n) = NK_NTOH16(__n); } while(0)

/**
 * Get the current program build date.
 */
#define NK_BUILD_DATE(__year, __month, __mday) \
	do {\
		NK_Char build_date[64];\
		NK_PChar month_name[] = {"Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"};\
		NK_Int i = 0;\
		snprintf(build_date, sizeof(build_date), "%s", __DATE__);\
		(__year) = atoi(&build_date[7]);\
		(__month) = 0;\
		(__mday) = atoi(&build_date[4]);\
		for (i = 0; i < sizeof(month_name) / sizeof(month_name[0]); ++i) {\
			if (NK_PREFIX_STRCASECMP(build_date, month_name[i])) {\
				(__month) = i + 1;\
				break;\
			}\
		}\
	} while (0)


/**
 * Get the current program build time.
 */
#define NK_BUILD_TIME(__hour, __min, __sec) \
	do {\
		NK_Char build_time[64];\
		snprintf(build_time, sizeof(build_time), "%s", __TIME__);\
		(__hour) = atoi(&build_time[0]);\
		(__min) = atoi(&build_time[3]);\
		(__sec) = atoi(&build_time[6]);\
	} while (0)


/// @brief Returns the minimum of two numbers.
#define NK_MIN(__a, __b) ((__a) <= (__b) ? (__a) : (__b))

/// @brief Returns the maximum of two numbers.
#define NK_MAX(__a, __b) ((__a) >= (__b) ? (__a) : (__b))

/**
 * Print hex data to terminal.
 */
#define NK_HEX_DUMP(__binary, __len) \
	do {\
		NK_Int i, line = 0;\
		NK_PByte byte = (NK_PByte)(__binary);\
		while (byte < (NK_PByte)(__binary) + (__len)) {\
			NK_Char plain[32];\
			snprintf(plain, sizeof(plain), "................");\
			printf("    %08x0    ", line++);\
			for (i = 0; i < 16; ++i) {\
				if (i < (NK_Int)((NK_PByte)(__binary) - byte + (__len))) {\
					NK_Byte byt = (NK_Char)(byte[i]);\
					printf(" %02x", (NK_UInt32)(byt));\
					if (byt >= 0x21 && byt <= 0x7f) {\
						plain[i] = (NK_Char)(byt);\
					}\
				} else {\
					printf("   ");\
				}\
			}\
			printf("    | %s | \r\n", plain);\
			byte += 16;\
		}\
	} while(0)

NK_CPP_EXTERN_END
#endif /* NK_MACRO_H_ */
