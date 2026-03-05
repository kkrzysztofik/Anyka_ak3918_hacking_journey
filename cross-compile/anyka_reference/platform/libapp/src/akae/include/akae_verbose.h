/**
 * Log information control singleton module.
 */

#include <akae_typedef.h>

#if !defined(AKAE_VERBOSE_H_)
#define AKAE_VERBOSE_H_
AK_C_HEADER_EXTERN_C_BEGIN


/**
 * Terminal output table handle.
 * Stores the context during the table output process.
 */
typedef struct _AK_VerboseForm {

	AK_char title[32];

	/**
	 * Table width.
	 */
	AK_size width, full_width;

	/**
	 * Left margin padding.
	 */
	AK_size padding;

	/**
	 * End-of-line marker.
	 */
	AK_boolean end_ln;

} AK_VerboseForm;

/**
 * Begin rendering a terminal table.
 * Initializes the @ref Form handle.
 */
AK_API AK_int akae_verbose_form_init (AK_VerboseForm *Form, AK_chrcptr title, AK_size width, AK_size padding);

/**
 * Append a plain-text row to the terminal table.
 *
 * @param[in]			end_ln			End-of-line marker; when True, a bottom border is appended after the plain text.
 * @param[in]			fmt				Text output format.
 *
 * @retval 0		Output succeeded.
 * @retval -1		Output failed.
 *
 */
AK_API AK_int akae_verbose_form_put_text (AK_VerboseForm *Form, AK_boolean end_ln, AK_chrptr fmt, ...);

/**
 * Append a "key - value" formatted text row to the terminal table.
 *
 * @param[in]			end_ln			End-of-line marker; when True, a bottom border is appended after the plain text.
 * @param[in]			key				Key text.
 * @param[in]			fmt				Value text output format.
 *
 * @retval 0		Output succeeded.
 * @retval -1		Output failed.
 *
 */
AK_API AK_int akae_verbose_form_put_kv (AK_VerboseForm *Form, AK_boolean end_ln, AK_chrcptr key, AK_chrcptr fmt, ...);

/**
 * Quick macro to output a "key - 32-bit integer" formatted row to the terminal table.
 *
 */
#define akae_verbose_form_put_key_int32(__Form, __end_ln, __key, __int32) \
	akae_verbose_form_put_kv (__Form, __end_ln, __key, "%d - %08x", (AK_int)(__int32), (AK_UInt32)(__int32))

/**
 * Quick macro to output a "key - string" formatted row to the terminal table.
 *
 */
#define akae_verbose_form_put_key_int64(__Form, __end_ln, __key, __int64) \
	akae_verbose_form_put_kv (__Form, __end_ln, __key, "%lld - %016x", (AK_int64)(__int64), (AK_UInt64)(__int64))

/**
 * Quick macro to output a "key - string" formatted row to the terminal table.
 *
 */
#define akae_verbose_form_put_key_string(__Form, __end_ln, __key, __str) \
	akae_verbose_form_put_kv (__Form, __end_ln, __key, "%s", __str)


/**
 * Finish rendering.\n
 * Calling this interface appends a bottom border based on the previous output and resets the handle.
 *
 */
AK_API AK_int akae_verbose_form_finish (AK_VerboseForm *Form);



AK_C_HEADER_EXTERN_C_END
#endif ///< AKAE_VERBOSE_H_
