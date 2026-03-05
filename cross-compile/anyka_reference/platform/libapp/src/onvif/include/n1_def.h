/*
 * N1 protocol data structure definitions.
 *
 */

#include <stdio.h>
#include <NkUtils/types.h>
#include <NkUtils/log.h>
#include <NkUtils/assert.h>
#include <NkUtils/macro.h>

#ifndef NK_N1_DEF_H_
#define NK_N1_DEF_H_
NK_CPP_EXTERN_BEGIN


/**
 * @macro
 *  Define the maximum number of channels.
 */
#define NK_N1_DEV_MAX_MEDIA_CH           (64)

/**
 * @macro
 *  Define the maximum number of streams.
 */
#define NK_N1_DEV_MAX_MEDIA_CH_STREAM    (4)

/**
 * @macro
 *  Define the maximum number of on-demand streams.
 */
#define NK_N1_DEV_MAX_MEDIA_STREAM_ONCMD (16)


/**
 * @macro
 *  32-bit integer identifier code conversion.
 */
#define NK_N1_UINT32_SYM(__n1, __n2, __n3, __n4) (((__n1)<<24)|((__n2)<<16)|((__n3)<<8)|((__n4)<<0))


#define NK_N1_H264_PROFILE_BASELINE (1)
#define NK_N1_H264_PROFILE_MAIN (2)
#define NK_N1_H264_PROFILE_HIGH (3)
/**
 * Maximum length of property option string.
 */
#define NK_N1_PROP_STR_MAX_LEN (128)

/**
 * Maximum number of property options.
 */
#define NK_N1_PROP_OPT_MAX_ENT (32)

/**
 * @brief Property type definition.
 */
typedef enum Nk_N1PropType
{

	NK_N1_PROP_TYPE_UNDEF = (-1),

	/**
	 * Boolean property.
	 */
	NK_N1_PROP_TYPE_BOOL = 0,
	/**
	 * Integer property.
	 */
	NK_N1_PROP_TYPE_INT,
	/**
	 * 64-bit integer property.
	 */
	NK_N1_PROP_TYPE_INT64,
	/**
	 * Enum property.
	 */
	NK_N1_PROP_TYPE_ENUM,
	/**
	 * Float property.
	 */
	NK_N1_PROP_TYPE_FLOAT,
	/**
	 * String property.
	 */
	NK_N1_PROP_TYPE_STRING,
	/**
	 * Hardware address property.
	 */
	NK_N1_PROP_TYPE_HWADDR,
	/**
	 * IPv4 address property.
	 */
	NK_N1_PROP_TYPE_IPV4,

} NK_N1PropType;


/**
 * Boolean property data structure.
 */
typedef struct Nk_N1PropBoolean
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	NK_Boolean val;

} NK_N1PropBoolean;

/**
 * Check if property data structure is valid.
 */
static inline NK_PChar
NK_N1_PROP_BOOL_CHECK(NK_N1PropBoolean *(__Prop))
{
	if (NK_True != (__Prop)->val && NK_False != (__Prop)->val) {
		return "Value Error.";
	}
	return NK_Nil;
}

/**
 * Print NK_N1PropBoolean data structure to terminal.
 * Mainly used for debugging.
 */
static inline NK_Void
NK_N1_PROP_BOOL_DUMP(NK_N1PropBoolean *(__Prop), const NK_PChar val_name)
{
	NK_TermTable Table;
	NK_PChar err = NK_Nil;

	NK_TermTbl_BeginDraw(&Table, "", 32, 4);
	err = NK_N1_PROP_BOOL_CHECK((__Prop));
	if (err) {
		NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);
	}
	NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%s", (__Prop)->val ? "True" : "False");
	NK_TermTbl_EndDraw(&Table);
}

/**
 * @brief Integer property data structure.
 */
typedef struct Nk_N1PropInteger
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	/**
	 * Property value.
	 */
	NK_Int32 val;

	/**
	 * Default value.\n
	 * In some cases, when the property value is incorrect, the default may serve as a reference for adjustment.
	 */
	NK_Int32 def;

	/**
	 * Maximum and minimum values of the property.\n
	 * When both max and min are 0, it means the property value is not constrained by max/min.\n
	 * Otherwise, the value and default value must be between max and min for the property to be valid.
	 */
	NK_Int min, max;

	/**
	 * Property options data structure.\n
	 * When Option is not Nil, @ref min and @ref max values are invalid.\n
	 * The range of values is based on the options.
	 */
	struct {
		NK_Size entries;
		NK_Int32 opt[NK_N1_PROP_OPT_MAX_ENT];
	} _Option, *Option;

} NK_N1PropInteger;

/**
 * Check if property data structure is valid.
 */
static inline NK_PChar
NK_N1_PROP_INT_CHECK(NK_N1PropInteger *(__Prop))
{
	NK_Int i = 0;

	if (!(__Prop)->Option) {
		/// Check if variables and default values are within range.
		if (!(0 == (__Prop)->min && 0 == (__Prop)->max)) {
			if ((__Prop)->min >= (__Prop)->max) {
				return "Range Error.";
			}
			if ((__Prop)->val > (__Prop)->max || (__Prop)->val < (__Prop)->min) {
				return "Value NOT in Range.";
			}
//			if ((__Prop)->def > (__Prop)->max || (__Prop)->def < (__Prop)->min) {
//				return "Default Value NOT in Range.";
//			}
		}
	} else {
		if ((__Prop)->Option->entries > sizeof((__Prop)->Option->opt) / sizeof((__Prop)->Option->opt[0])
				|| !(__Prop)->Option->entries) {

			return "Options Entires Error.";
		}
		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {
			if ((__Prop)->Option->opt[i] == (__Prop)->val) {
				break;
			}
		}
		if (i == (__Prop)->Option->entries) {
			return "Value NOT in Options.";
		}
		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {
			if ((__Prop)->Option->opt[i] == (__Prop)->def) {
				break;
			}
		}
		if (i == (NK_Int)(__Prop)->Option->entries) {
			return "Default Value NOT in Options.";
		}
	}
	return NK_Nil;
}

/**
 * Print NK_N1PropInteger data structure to terminal.
 * Mainly used for debugging.
 */
#define NK_N1_PROP_INT_DUMP(__Prop, val_name) \
	do {\
		NK_Int i = 0;\
		NK_TermTable Table;\
		NK_PChar err = NK_Nil;\
		NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property Integer" : (val_name), 32, 4);\
		err = NK_N1_PROP_INT_CHECK((__Prop));\
		if (err) {\
			NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);\
		}\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%d", (__Prop)->val);\
		if (NK_Nil != (__Prop)->Option) {\
			NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%d", (__Prop)->def);\
			NK_TermTbl_PutText(&Table, NK_True, "%-28s", "Options");\
			for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {\
				NK_TermTbl_PutText(&Table, NK_False, "%28d", (__Prop)->Option->opt[i]);\
			}\
		} else {\
			NK_TermTbl_PutKeyValue(&Table, NK_True, "Minimum", "%d", (__Prop)->min);\
			NK_TermTbl_PutKeyValue(&Table, NK_True, "Maximum", "%d", (__Prop)->max);\
			NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%d", (__Prop)->def);\
		}\
		NK_TermTbl_EndDraw(&Table);\
	} while (0)

/**
 * @brief 64-bit integer property data structure.
 */
typedef struct Nk_N1PropInt64
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	NK_Int64 val, min, max, def;

	struct {
		NK_Size entries;
		NK_Int64 opt[NK_N1_PROP_OPT_MAX_ENT];
	} _Option, *Option;

} NK_N1PropInt64;

/**
 * Data structure check.
 */
static inline NK_PChar
NK_N1_PROP_INT64_CHECK(NK_N1PropInt64 *(__Prop))
{
        NK_INT i;
	/// Determine if the value is within the corresponding range
        if ((__Prop)->Option == NK_Nil){
          if (!(0 == (__Prop)->min && 0 == (__Prop)->max)) {
           if ((__Prop)->min > (__Prop)->max){
              return "Range Error.";
           }
           if ((__Prop)->val > (__Prop)->max || (__Prop)->val < (__Prop)->min){
              return "Value Not In Range.";
           }
//           if ((__Prop)->def > (__Prop)->max || (__Prop)->def < (__Prop)->min){
//             return "Default Value Not In  Range.";
//          }
          }
        }else{
           if ((__Prop)->Option->entries > (sizeof((__Prop)->Option->opt)/sizeof((__Prop)->Option->opt[0]))){
              return "Range error!";
           }
           for (i = 0; i < (NK_Int)(__Prop)->Option->entries; i++){
              if ((__Prop)->Option->opt[i] == (__Prop)->val){
                 break;
              }
           }
           if (i == (__Prop)->Option->entries){
              return "Value Not In Options.";
           }

           for (i = 0; i < (NK_Int)(__Prop)->Option->entries; i++){
              if ((__Prop)->Option->opt[i] == (__Prop)->def){
                 break;
              }
           }
           if (i == (__Prop)->Option->entries){
              return "Default Value Not In Options.";
           }
        }

	return NK_Nil;
}

/**
 * Print NK_N1PropInteger data structure to terminal.
 * Mainly used for debugging.
 */
static inline NK_Void
NK_N1_PROP_INT64_DUMP(NK_N1PropInt64 *(__Prop), const NK_PChar val_name)
{
//	NK_Int i = 0;
//	NK_TermTable Table;
//	NK_PChar err = NK_Nil;
//
//	NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property Int64" : (val_name), 32, 4);
//
//	err = NK_N1_PROP_INT64_CHECK((__Prop));
//	if (err) {
//		NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);
//	}
//	NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%lld", (__Prop)->val);
//	if (NK_Nil != (__Prop)->Option) {
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%lld", (__Prop)->def);
//		NK_TermTbl_PutText(&Table, NK_True, "%-28s", "Options");
//		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {
//			NK_TermTbl_PutText(&Table, NK_False, "%28lld", (__Prop)->Option->opt[i]);
//		}
//	} else {
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Minimum", "%lld", (__Prop)->min);
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Maximum", "%lld", (__Prop)->max);
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%lld", (__Prop)->def);
//	}
//	NK_TermTbl_EndDraw(&Table);
}

/**
 * @brief Enum property data structure.
 *
 * Enum properties are similar to integer properties.\n
 * Unlike integer properties, enum properties have no max/min limits but must have options.\n
 * Each enum option value must have a corresponding text.
 */
typedef struct Nk_N1PropEnum
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	/**
	 * Property value and default value.
	 */
	NK_UInt32 val, def;

	struct {
		/**
		 * Number of valid options.
		 */
		NK_Size entries;
		/**
		 * Option values.
		 */
		NK_UInt32 opt[NK_N1_PROP_OPT_MAX_ENT];
		/**
		 * Text corresponding to the option values.
		 */
		NK_PChar str[NK_N1_PROP_OPT_MAX_ENT];

	} _Option, *Option;

} NK_N1PropEnum;

/**
 * Data structure check.
 */
static inline NK_PChar
NK_N1_PROP_ENUM_CHECK(NK_N1PropEnum *(__Prop))
{
	NK_Int i = 0;;
	/// Enum structure data value condition judgment
	if (!(__Prop)->Option) {
		return "Option NULL";
	}else{
		if (!((__Prop)->Option->entries > 0
				&& (__Prop)->Option->entries < (sizeof((__Prop)->Option->opt)/sizeof((__Prop)->Option->opt[0])))) {
			return "Option Range Error.";
		}
		/// Traverse all enum options; each enum value should not correspond to an empty string.
		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {
			if (!(__Prop)->Option->str[i]) {
				return "Option Text Error.";
			}
		}
		/// Traverse all enum options to ensure the value is within the enum options.
		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; i++) {
			if ((__Prop)->Option->opt[i] == (__Prop)->val) {
				break;
			}
		}
		if (i == (__Prop)->Option->entries) {
			return "Value Not In Options.";
		}
	}
	return NK_Nil;
}
/**
 * Print NK_N1PropEnum data structure to terminal.
 * Mainly used for debugging.
 */
static inline NK_Void
NK_N1_PROP_ENUM_DUMP(NK_N1PropEnum *(__Prop), const NK_PChar val_name)
{
//	NK_Int i = 0;
//	NK_TermTable Table;
//	NK_PChar err = NK_Nil;
//
//	NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property Enum" : (val_name), 32, 4);
//
//	err = NK_N1_PROP_ENUM_CHECK((__Prop));
//	if (err) {
//		NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);
//	}
//	NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%u", (__Prop)->val);
//	if (NK_Nil != (__Prop)->Option) {
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%u", (__Prop)->def);
//		NK_TermTbl_PutText(&Table, NK_True, "%-28s", "Options");
//		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {
//			NK_TermTbl_PutText(&Table, NK_False, "%u: %s",
//					(__Prop)->Option->opt[i], (__Prop)->Option->str[i]);
//		}
//	}
//	NK_TermTbl_EndDraw(&Table);
}

/**
 * @brief Float property data structure.
 */
typedef struct Nk_N1PropFloat
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	NK_DFloat val, min, max, def;

	struct {
		NK_Size entries;
		NK_DFloat opt[NK_N1_PROP_OPT_MAX_ENT];
	} _Option, *Option;

} NK_N1PropFloat;

/**
 * Data structure check.
 */
static inline NK_PChar
NK_N1_PROP_FLOAT_CHECK(NK_N1PropFloat *(__Prop))
{
        NK_INT i;
	/// Float data structure value judgment
        if ((__Prop)->Option == NK_Nil) {
          if (!(0 == (__Prop)->min && 0 == (__Prop)->max)) {
           if ((__Prop)->min > (__Prop)->max) {
              return "Range error.";
           }
           if ((__Prop)->val > (__Prop)->max || (__Prop)->val < (__Prop)->min) {
              return "Value Not In Range.";
           }
//           if ((__Prop)->def > (__Prop)->max || (__Prop)->def < (__Prop)->min) {
//              return "DefValue Not In Range.";
//          }
          }
        }else{
           if ((__Prop)->Option->entries > (sizeof((__Prop)->Option->opt)/sizeof((__Prop)->Option->opt[0]))) {
              return  "Range Error.";
           }
           for (i = 0; i < (NK_Int)(__Prop)->Option->entries; i++) {
              if ((__Prop)->Option->opt[i] == (__Prop)->val) {
                 break;
              }
           }
           if (i == (__Prop)->Option->entries) {
              return "Value Not In Options.";
           }

           for (i = 0; i < (NK_Int)(__Prop)->Option->entries; i++) {
              if ((__Prop)->Option->opt[i] == (__Prop)->def) {
                 break;
              }
           }
           if (i == (__Prop)->Option->entries) {
              return "Default Value Not In Options.";
           }
        }
	return NK_Nil;
}

/**
 * Print NK_N1PropFloat data structure to terminal.
 * Mainly used for debugging.
 */
static inline
NK_Void NK_N1_PROP_FLOAT_DUMP(NK_N1PropFloat *(__Prop), const NK_PChar val_name)
{
//	NK_Int i = 0;
//	NK_TermTable Table;
//	NK_PChar err = NK_Nil;
//
//	NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property Float" : (val_name), 32, 4);
//
//	err = NK_N1_PROP_FLOAT_CHECK((__Prop));
//	if (err) {
//		NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);
//	}
//	NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%.5f", (__Prop)->val);
//	if (NK_Nil != (__Prop)->Option) {
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%.5f", (__Prop)->def);
//		NK_TermTbl_PutText(&Table, NK_True, "%-28s", "Options");
//		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {
//			NK_TermTbl_PutText(&Table, NK_False, "%22.5f", (__Prop)->Option->opt[i]);
//		}
//	} else {
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Minimum", "%.5f", (__Prop)->min);
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Maximum", "%.5f", (__Prop)->max);
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%.5f", (__Prop)->def);
//	}
//	NK_TermTbl_EndDraw(&Table);
}

/**
 * Text property data structure.
 */
typedef struct Nk_N1PropString
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	NK_Char val[NK_N1_PROP_STR_MAX_LEN + 1], def[NK_N1_PROP_STR_MAX_LEN + 1];

	/**
	 * Maximum length of the text, different from @ref NK_N1_PROP_STR_MAX_LEN.
	 * The text is valid only if its length is within this limit.
	 */
	NK_Size max_len;

	/**
	 * Refer to NK_N1PropInteger::Option.
	 */
	struct {
		NK_Size entries;
		NK_PChar opt[NK_N1_PROP_OPT_MAX_ENT];
	} _Option, *Option;

} NK_N1PropString;

/**
 * Set string property.
 */
#define NK_N1_PROP_STR_SET(__Prop, __str) \
	do{\
		NK_Int i = 0;\
		if (NK_Nil != (__str)) {\
			if (NK_Nil != (__Prop)->Option) {\
				/** Must comply with the data in the options. */\
				for (i = 0; i < (__Prop)->Option->entries; ++i) {\
					if (NK_Nil != (__Prop)->Option->opt[i] && NK_STRCMP((__Prop)->Option->opt[i], (__str))) {\
						(__Prop)->max_len = 0;\
						snprintf((__Prop)->val, sizeof((__Prop)->val), "%s", (__str));\
					}\
				}\
			} else {\
				snprintf((__Prop)->val, 0 == (__Prop)->max_len ? sizeof((__Prop)->val) : (__Prop)->max_len, "%s", (__str));\
			}\
		}\
	} while(0)

/**
 * Data structure check.
 */
static inline NK_PChar
NK_N1_PROP_STR_CHECK(NK_N1PropString *(__Prop))
{
        NK_INT i;
	/// Check string value conditions and range
        if ((__Prop)->max_len < 0) {
           return "String Length Value Error.";
        }
        if ((__Prop)->max_len < strlen((__Prop)->val)) {
           return "Value String Length Error.";
        }
        if ((__Prop)->max_len < strlen((__Prop)->def)) {
           return "Default String Length Error.";
        }

        if ((__Prop)->Option->entries > (sizeof((__Prop)->Option->opt)/sizeof((__Prop)->Option->opt[0]))) {
           return "Range Error.";
        }

        for (i = 0; i < (NK_Int)(__Prop)->Option->entries; i++) {
           if (strlen((__Prop)->Option->opt[i]) <= (__Prop)->max_len) {
              if (strcmp((__Prop)->Option->opt[i], (__Prop)->val) == 0) {
                 break;
              }
           }
           else{
              return "Value String Length Error.";
           }
        }

        if (i == (__Prop)->Option->entries) {
           return "Value Not In Options.";
        }

        for (i = 0;i < (NK_Int)(__Prop)->Option->entries; i++) {
           if (strlen((__Prop)->Option->opt[i]) <= (__Prop)->max_len) {
              if (strcmp((__Prop)->Option->opt[i], (__Prop)->def) == 0) {
                 break;
              }
           }
           else{
              return "Defaule Value String Length Error.";
           }
        }
        if (i == (__Prop)->Option->entries) {
           if (i == (__Prop)->Option->entries) {
              return "Default Value Not In Options.";
           }
        }

	return NK_Nil;
}

/**
 * Print NK_N1PropString data structure to terminal.
 * Mainly used for debugging.
 */
static inline
NK_Void NK_N1_PROP_STRING_DUMP(NK_N1PropString *(__Prop), const NK_PChar val_name)
{
//	NK_Int i;
//	NK_TermTable Table;
//	NK_PChar err = NK_Nil;
//	NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property String" : (val_name), 32, 4);
//	/// Judge the value and range of string data types
//	err = NK_N1_PROP_STR_CHECK((__Prop));
//	if (err) {
//		NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);
//	}
//	NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%s", (__Prop)->val);
//	if ((__Prop)->Option != NK_Nil) {
//		NK_TermTbl_PutKeyValue(&Table, NK_True, "Default", "%s", (__Prop)->def);
//		NK_TermTbl_PutText(&Table, NK_True, "%s", "Options");
//		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; i++) {
//			NK_TermTbl_PutText(&Table, NK_False, "%s", (__Prop)->Option->opt[i]);
//		}
//	}
//	NK_TermTbl_EndDraw(&Table);
}

/**
 * @brief Device hardware address property data structure.
 */
typedef struct Nk_N1PropHwAddr
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	NK_UInt8 val[6];

} NK_N1PropHwAddr;

/**
 * Set hardware address property.
 */
#define NK_N1_PROP_HWADDR_SET(__Prop, __hw0, __hw1, __hw2, __hw3, __hw4, __hw5) \
	do{\
		(__Prop)->val[0] = (NK_UInt32)(__hw0);\
		(__Prop)->val[1] = (NK_UInt32)(__hw1);\
		(__Prop)->val[2] = (NK_UInt32)(__hw2);\
		(__Prop)->val[3] = (NK_UInt32)(__hw3);\
		(__Prop)->val[4] = (NK_UInt32)(__hw4);\
		(__Prop)->val[5] = (NK_UInt32)(__hw5);\
	} while(0)

/**
 * Convert text to hardware address property.
 */
#define NK_N1_PROP_HWADDR_ATON(__Prop, __hw_text) \
	do{\
		NK_Size len = strlen(__hw_text);\
		NK_Char *chr, ch;\
		NK_Int i = 0, ii = 0;\
		for (i = 0; i < 6; ++i) {\
			chr = __hw_text + i * 3;\
			if (chr < (__hw_text) + len) {\
				for (ii = 0; ii < 2; ++ii) {\
					if ((chr[ii] >= '0' && chr[ii] <= '9') \
							|| (chr[ii] >= 'a' && chr[ii] <= 'f') \
							|| (chr[ii] >= 'A' && chr[ii] <= 'F')) {\
						if (chr[ii] >= 'a' && chr[ii] <= 'f') ch = chr[ii] - 'a' + 10;\
						else if (chr[ii] >= 'A' && chr[ii] <= 'F') ch = chr[ii] - 'A' + 10;\
						else if (chr[ii] >= '0' && chr[ii] <= '9') ch = chr[ii] - '0';\
						else ch = 0;\
						\
						if (0 == ii) {\
							(__Prop)->val[i] = (ch << 4);\
						} else {\
							(__Prop)->val[i] |= ch;\
						}\
					}\
				}\
			}\
		}\
	} while(0);

/**
 * Convert hardware address property to text.
 */
#define NK_N1_PROP_HWADDR_NTOA(__Prop, __text, __size) \
	snprintf(__text, (__size), "%02x:%02x:%02x:%02x:%02x:%02x",\
		(NK_UInt32)((__Prop)->val[0]),\
		(NK_UInt32)((__Prop)->val[1]),\
		(NK_UInt32)((__Prop)->val[2]),\
		(NK_UInt32)((__Prop)->val[3]),\
		(NK_UInt32)((__Prop)->val[4]),\
		(NK_UInt32)((__Prop)->val[5]))

#define NK_N1_PROP_HWADDR_STR NK_N1_PROP_HWADDR_NTOA

/**
 * Data structure check.
 */
static inline NK_PChar
NK_N1_PROP_HWADDR_CHECK(NK_N1PropHwAddr *(__Prop))
{
	if ((0 == (__Prop)->val[0] && 0 == (__Prop)->val[1]
			&& 0 == (__Prop)->val[2] && 0 == (__Prop)->val[3]
			&& 0 == (__Prop)->val[4] && 0 == (__Prop)->val[5])
			|| (0xff == (__Prop)->val[0] && 0xff == (__Prop)->val[1]
					&& 0xff == (__Prop)->val[2] && 0xff == (__Prop)->val[3]
					&& 0xff == (__Prop)->val[4] && 0xff == (__Prop)->val[5])) {
		return "Invalid Address.";
	}
	return NK_Nil;
}

/**
 * Print NK_N1PropHwAddr data structure to terminal.
 * Mainly used for debugging.
 */
static inline NK_Void
NK_N1_PROP_HWADDR_DUMP(NK_N1PropHwAddr *(__Prop), const NK_PChar val_name)
{
//	NK_TermTable Table;
//	NK_PChar err = NK_Nil;
//	NK_Char text[32];
//
//	NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property Hardware Address" : (val_name), 64, 4);
//	err = NK_N1_PROP_HWADDR_CHECK((__Prop));
//	if (err) {
//		NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);
//	}
//	NK_N1_PROP_HWADDR_NTOA((__Prop), text, sizeof(text));
//	NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%s", text);
//	NK_TermTbl_EndDraw(&Table);
}


/**
 * @brief IPv4 address property data structure.
 */
typedef struct Nk_N1PropIPv4
{
	/**
	 * Property read-only flag.
	 */
	NK_Boolean read_only;

	/**
	 * Property type.
	 */
	NK_N1PropType type;

	NK_UInt8 val[4];

} NK_N1PropIPv4;

/**
 * Set IPv4 address property.
 */
#define NK_N1_PROP_IPV4_SET(__Prop, __ip0, __ip1, __ip2, __ip3) \
	do{\
		(__Prop)->val[0] = (NK_UInt32)(__ip0);\
		(__Prop)->val[1] = (NK_UInt32)(__ip1);\
		(__Prop)->val[2] = (NK_UInt32)(__ip2);\
		(__Prop)->val[3] = (NK_UInt32)(__ip3);\
	} while(0)

/**
 * Convert text to IPv4 address property setting.
 */
#define NK_N1_PROP_IPV4_ATON(__Prop, __ipv4_text) \
	do{\
		NK_Char *ip0, *ip1, *ip2, *ip3;\
		ip0 = __ipv4_text;\
		if (NK_Nil != ip0) {\
			ip1 = strchr(ip0, '.');\
			if (NK_Nil != ip1++) {\
				ip2 = strchr(ip1, '.');\
				if (NK_Nil != ip2++) {\
					ip3 = strchr(ip2, '.');\
					if (NK_Nil != ip3++) {\
						(__Prop)->val[0] = atoi(ip0);\
						(__Prop)->val[1] = atoi(ip1);\
						(__Prop)->val[2] = atoi(ip2);\
						(__Prop)->val[3] = atoi(ip3);\
					}\
				}\
			}\
		}\
	} while(0);


/**
 * IPv4 address property convert to text.
 */
#define NK_N1_PROP_IPV4_NTOA(__Prop, __text, __size) \
	snprintf(__text, (__size), "%d.%d.%d.%d",\
		(NK_Int)((__Prop)->val[0]),\
		(NK_Int)((__Prop)->val[1]),\
		(NK_Int)((__Prop)->val[2]),\
		(NK_Int)((__Prop)->val[3]))

#define NK_N1_PROP_IPV4_STR NK_N1_PROP_IPV4_NTOA

/**
 * Data structure check.
 */
static inline NK_PChar
NK_N1_PROP_IPV4_CHECK(NK_N1PropIPv4 *(__Prop))
{
	if (0 == (__Prop)->val[0] && 0 == (__Prop)->val[1]
			&& 0 == (__Prop)->val[2] && 0 == (__Prop)->val[3]) {
		return "Invalid Address.";
	}
	return NK_Nil;
}

/**
 * Print NK_N1PropIPv4 data structure to terminal.
 * Mainly used for debugging.
 */
static inline NK_Void
NK_N1_PROP_IPv4_DUMP(NK_N1PropIPv4 *(__Prop), const NK_PChar val_name)
{
//	NK_TermTable Table;
//	NK_PChar err = NK_Nil;
//
//	NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property IPv4" : (val_name), 32, 4);
//	err = NK_N1_PROP_IPV4_CHECK((__Prop));
//	if (err) {
//		NK_TermTbl_PutText(&Table, NK_True, "Error: %s", err);
//	}
//	NK_TermTbl_PutKeyValue(&Table, NK_True, "Value", "%d.%d.%d.%d",
//			(__Prop)->val[0], (__Prop)->val[1], (__Prop)->val[2], (__Prop)->val[3]);
//	NK_TermTbl_EndDraw(&Table);
}

/**
 * Property collection.
 */
typedef struct Nk_N1Property
{
	union {

		struct {

			/**
			 * Property read-only flag.
			 */
			NK_Boolean read_only;

			/**
			 * Property type.
			 */
			NK_N1PropType type;
		};

		NK_N1PropBoolean Boolean;
		NK_N1PropInteger Integer;
		NK_N1PropInt64 Integer64;
		NK_N1PropEnum Enum;
		NK_N1PropFloat Float;
		NK_N1PropString String;
		NK_N1PropHwAddr HwAddr;
		NK_N1PropIPv4 IPv4;
	};

} NK_N1Property;

/**
 * Data structure check.
 */
#define NK_N1_PROP_CHECK(__Prop) \
	((NK_N1_PROP_TYPE_BOOL == (__Prop)->type) \
	 ? NK_N1_PROP_BOOL_CHECK(&(__Prop)->Boolean) \
			:(NK_N1_PROP_TYPE_INT == (__Prop)->type) \
			 ?  NK_N1_PROP_INT_CHECK(&(__Prop)->Integer) \
					 :(NK_N1_PROP_TYPE_INT64 == (__Prop)->type) \
					  ? NK_N1_PROP_INT64_CHECK(&(__Prop)->Integer64) \
							  :(NK_N1_PROP_TYPE_ENUM == (__Prop)->type) \
							   ? NK_N1_PROP_ENUM_CHECK(&(__Prop)->Enum) \
									   :(NK_N1_PROP_TYPE_FLOAT == (__Prop)->type) \
										? NK_N1_PROP_FLOAT_CHECK(&(__Prop)->Float) \
												:(NK_N1_PROP_TYPE_STRING == (__Prop)->type) \
												 ? NK_N1_PROP_STRING_CHECK(&(__Prop)->String) \
														 :(NK_N1_PROP_TYPE_HWADDR == (__Prop)->type) \
														  ? NK_N1_PROP_HWADDR_CHECK(&(__Prop)->HwAddr) \
																  :(NK_N1_PROP_TYPE_IPV4 == (__Prop)->type) \
																   ? NK_N1_PROP_IPV4_CHECK(&(__Prop)->IPv4) \
																		   : "Invalid Property.")

/**
 * Print NK_N1PropSet data structure to terminal.
 * Mainly used for debugging.
 */
#define NK_N1_PROP_DUMP(__Prop, __val_name) \
	do{\
		if (NK_N1_PROP_TYPE_BOOL == (__Prop)->type) {\
			NK_N1_PROP_BOOL_DUMP(&(__Prop)->Boolean, __val_name);\
		} else if (NK_N1_PROP_TYPE_INT == (__Prop)->type) {\
			NK_N1_PROP_INT_DUMP(&(__Prop)->Integer, __val_name);\
		} else if (NK_N1_PROP_TYPE_INT64 == (__Prop)->type) {\
			NK_N1_PROP_INT64_DUMP(&(__Prop)->Integer64, __val_name);\
		} else if (NK_N1_PROP_TYPE_ENUM == (__Prop)->type) {\
			NK_N1_PROP_ENUM_DUMP(&(__Prop)->Enum, __val_name);\
		} else if (NK_N1_PROP_TYPE_FLOAT == (__Prop)->type) {\
			NK_N1_PROP_FLOAT_DUMP(&(__Prop)->Float, __val_name);\
		} else if (NK_N1_PROP_TYPE_STRING == (__Prop)->type) {\
			NK_N1_PROP_STRING_DUMP(&(__Prop)->String, __val_name);\
		} else if (NK_N1_PROP_TYPE_HWADDR == (__Prop)->type) {\
			NK_N1_PROP_HWADDR_DUMP(&(__Prop)->HwAddr, __val_name);\
		} else if (NK_N1_PROP_TYPE_IPV4 == (__Prop)->type) {\
			NK_N1_PROP_IPV4_DUMP(&(__Prop)->IPv4, __val_name);\
		}\
	} while (0)

/**
 * Append an option to the property.
 */
#define NK_N1_PROP_ADD_OPT(__Prop, __val) \
	do{\
		if (!(__Prop)->Option){\
			(__Prop)->Option = &((__Prop)->_Option);\
			(__Prop)->Option->entries = 0;\
			(__Prop)->Option->opt[(__Prop)->Option->entries++] = (__val);\
		}\
		if ((__Prop)->Option->entries >= NK_N1_PROP_OPT_MAX_ENT){\
		/* Option quota is full. */\
			break;\
		}\
		NK_Size size;\
		for(size = 0; size < (__Prop)->Option->entries; size++)\
		{\
			/* Add only if the option does not exist */\
			if(__val == (__Prop)->Option->opt[size])\
			{\
				break;\
			}else{\
				if(size == (__Prop)->Option->entries - 1)\
				{\
					(__Prop)->Option->opt[(__Prop)->Option->entries++] = (__val);\
				}\
            }\
		}\
	} while (0)

/**
 * Append an enum option to the property.
 */
#define NK_N1_PROP_ADD_ENUM(__Prop, __type, __opt) \
	do{\
		NK_Size opt_entries = NK_Nil != (__Prop)->Option ? (__Prop)->Option->entries : 0;\
		NK_N1_PROP_ADD_OPT(__Prop, __opt);\
		if (opt_entries + 1 == (__Prop)->Option->entries)\
			(__Prop)->Option->str[(__Prop)->Option->entries - 1] = NK_ENUM_MAP(__type, __opt);\
	} while(0)


/**
 * Undefined enum variable.
 */
#define NK_ENUM_UNDEFINED ""

/**
 * Define an interface template for mapping enum values to a text set.
 * Defining functions through this interface effectively solves the problem of repeated naming definitions.
 * The macro definition will expand into relevant type interfaces during compilation.
 */
#define DECLARE_NK_ENUM_MAP(__type)  NK_PChar NK_Enum_Map##__type(NK_##__type enm)

/**
 * Define an interface template for mapping a text set to enum values.
 */
#define DECLARE_NK_ENUM_UNMAP(__type)  NK_##__type NK_Enum_Unmap##__type(NK_PChar text)


/**
 * Call the corresponding type's enum interface.
 */
#define NK_ENUM_MAP(__type, __enm) NK_Enum_Map##__type(__enm)

/**
 * Call the corresponding type's enum interface.
 */
#define NK_ENUM_UNMAP(__type, __text) NK_Enum_Unmap##__type(__text)


/**
 * Image size.
 */
#define NK_N1_IMG_SZ(__width, __height) ((NK_ALIGN_BIG_END(__width, 2) * 10000) + NK_ALIGN_BIG_END(__height, 2))
#define NK_N1_IMG_SZ_UNDEF      NK_N1_IMG_SZ(0, 0)
#define NK_N1_IMG_SZ_160X90     NK_N1_IMG_SZ(160, 90)
#define NK_N1_IMG_SZ_160X120    NK_N1_IMG_SZ(160, 120)
#define NK_N1_IMG_SZ_172X144    NK_N1_IMG_SZ(172, 144)
#define NK_N1_IMG_SZ_320X180    NK_N1_IMG_SZ(320, 180)
#define NK_N1_IMG_SZ_320X240    NK_N1_IMG_SZ(320, 240)
#define NK_N1_IMG_SZ_352X240    NK_N1_IMG_SZ(352, 240)
#define NK_N1_IMG_SZ_352X288    NK_N1_IMG_SZ(352, 288)
#define NK_N1_IMG_SZ_360X240    NK_N1_IMG_SZ(360, 240)
#define NK_N1_IMG_SZ_360X288    NK_N1_IMG_SZ(360, 288)
#define NK_N1_IMG_SZ_480X270    NK_N1_IMG_SZ(480, 270)
#define NK_N1_IMG_SZ_480X360    NK_N1_IMG_SZ(480, 360)
#define NK_N1_IMG_SZ_480X480    NK_N1_IMG_SZ(480, 480)
#define NK_N1_IMG_SZ_528X384    NK_N1_IMG_SZ(528, 384)
#define NK_N1_IMG_SZ_640X360    NK_N1_IMG_SZ(640, 360)
#define NK_N1_IMG_SZ_640X480    NK_N1_IMG_SZ(640, 480)
#define NK_N1_IMG_SZ_704X240    NK_N1_IMG_SZ(704, 240)
#define NK_N1_IMG_SZ_704X288    NK_N1_IMG_SZ(704, 288)
#define NK_N1_IMG_SZ_704X480    NK_N1_IMG_SZ(704, 480)
#define NK_N1_IMG_SZ_704X576    NK_N1_IMG_SZ(704, 576)
#define NK_N1_IMG_SZ_720X240    NK_N1_IMG_SZ(720, 240)
#define NK_N1_IMG_SZ_720X288    NK_N1_IMG_SZ(720, 288)
#define NK_N1_IMG_SZ_720X480    NK_N1_IMG_SZ(720, 480)
#define NK_N1_IMG_SZ_720X576    NK_N1_IMG_SZ(720, 576)
#define NK_N1_IMG_SZ_720X720    NK_N1_IMG_SZ(720, 720)
#define NK_N1_IMG_SZ_800X600    NK_N1_IMG_SZ(800, 600)
#define NK_N1_IMG_SZ_800X800    NK_N1_IMG_SZ(800, 800)
#define NK_N1_IMG_SZ_960X480    NK_N1_IMG_SZ(960, 480)
#define NK_N1_IMG_SZ_960X576    NK_N1_IMG_SZ(960, 576)
#define NK_N1_IMG_SZ_960X960    NK_N1_IMG_SZ(960, 960)
#define NK_N1_IMG_SZ_1280X720   NK_N1_IMG_SZ(1280, 720)
#define NK_N1_IMG_SZ_1280X960   NK_N1_IMG_SZ(1280, 960)
#define NK_N1_IMG_SZ_1280X1024  NK_N1_IMG_SZ(1280, 1024)
#define NK_N1_IMG_SZ_1056X1056  NK_N1_IMG_SZ(1280, 1024)
#define NK_N1_IMG_SZ_1600X900   NK_N1_IMG_SZ(1600, 900)
#define NK_N1_IMG_SZ_1600X1200  NK_N1_IMG_SZ(1600, 1200)
#define NK_N1_IMG_SZ_1920X1080  NK_N1_IMG_SZ(1920, 1080)
#define NK_N1_IMG_SZ_2048X1512  NK_N1_IMG_SZ(2048, 1512)
#define NK_N1_IMG_SZ_2048X1520  NK_N1_IMG_SZ(2048, 1520)
#define NK_N1_IMG_SZ_2048X1536  NK_N1_IMG_SZ(2048, 1536)
#define NK_N1_IMG_SZ_2304X1296  NK_N1_IMG_SZ(2304, 1296)
#define NK_N1_IMG_SZ_2304X1728  NK_N1_IMG_SZ(2304, 1728)
#define NK_N1_IMG_SZ_2560X1440  NK_N1_IMG_SZ(2560, 1440)
#define NK_N1_IMG_SZ_2592X1520  NK_N1_IMG_SZ(2592, 1520)
#define NK_N1_IMG_SZ_2592X1944  NK_N1_IMG_SZ(2592, 1944)
#define NK_N1_IMG_SZ_2688X1512  NK_N1_IMG_SZ(2688, 1512)
typedef NK_Size NK_N1ImageSize;


/**
 * Encoding bit rate control mode.
 */
typedef enum Nk_N1BitRateCtrlMode
{
	NK_N1_BR_CTRL_MODE_UNDEF = (-1),
	/**
	 * Constant Bit Rate (CBR) control.
	 */
	NK_N1_BR_CTRL_MODE_CBR,

	/**
	 * Variable Bit Rate (VBR) control.
	 */
	NK_N1_BR_CTRL_MODE_VBR,

} NK_N1BitRateCtrlMode;

/**
 * Audio input mode.
 */
typedef enum Nk_N1AudioInputMode
{
        NK_N1_AUDIO_INPUT_MODE_UNDEF = (-1),
        /**
         * Audio input method.
         */
        NK_N1_AUDIO_INPUT_MODE_AUTO,

        NK_N1_AUDIO_INPUT_MODE_LINE,

	    NK_N1_AUDIO_INPUT_MODE_MIC

} NK_N1AudioInputMode;

/**
 * Get the text information corresponding to the NK_N1BitRateCtrlMode enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1BitRateCtrlMode);

/**
 * Get the enum value corresponding to the NK_N1BitRateCtrlMode text information.
 */
extern DECLARE_NK_ENUM_UNMAP(N1BitRateCtrlMode);

/**
 * Get the text information corresponding to the NK_N1AudioInputMode enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1AudioInputMode);

/**
 * Get the enum value corresponding to the NK_N1AudioInputMode text information.
 */
extern DECLARE_NK_ENUM_UNMAP(N1AudioInputMode);

typedef enum Nk_N1VideoEncCodec
{
	NK_N1_VENC_CODEC_UNDEF = (-1),
	NK_N1_VENC_CODEC_MPEG,
	NK_N1_VENC_CODEC_H264,
	NK_N1_VENC_CODEC_HEVC,
	NK_N1_VENC_CODEC_H264_PLUS,
	NK_N1_VENC_CODEC_HEVC_PLUS,
} NK_N1VideoEncCodec;

/**
 * Get the text information corresponding to the Nk_N1VideoEncCodec enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1VideoEncCodec);

/**
 * Get the enum value corresponding to the Nk_N1VideoEncCodec text information.
 */
extern DECLARE_NK_ENUM_UNMAP(N1VideoEncCodec);

typedef enum Nk_N1AudioEncCodec
{
	NK_N1_AUDIO_CODEC_UNDEF = (-1),
	NK_N1_AUDIO_CODEC_G711A,
	NK_N1_AUDIO_CODEC_G711U,
	NK_N1_AUDIO_CODEC_AAC,

} NK_N1AudioEncCodec;

/**
 * Get the text information corresponding to the Nk_N1AudioEncCodec enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1AudioEncCodec);

/**
 * Get the enum value corresponding to the Nk_N1AudioEncCodec text information.
 */
extern DECLARE_NK_ENUM_UNMAP(N1AudioEncCodec);

typedef enum Nk_N1PTZCommand {

	NK_N1_PTZ_CMD_UNDEF			= (-1),

	NK_N1_PTZ_CMD_STOP			= (0),
	NK_N1_PTZ_CMD_CAMERA_PWRON  =(1),
	NK_N1_PTZ_CMD_LIGHT_PWRON       =(2),
	NK_N1_PTZ_CMD_WIPER_PWRON     = (3),
	NK_N1_PTZ_CMD_FAN_PWRON	= (4),
	NK_N1_PTZ_CMD_HEATER_PWRON	= (5),
	NK_N1_PTZ_CMD_AUX_PWRON1	= (6),
	NK_N1_PTZ_CMD_AUX_PWRON2	= (7),

	NK_N1_PTZ_CMD_TILT_UP			= (100),
	NK_N1_PTZ_CMD_TILT_DOWN,

	NK_N1_PTZ_CMD_PAN_LEFT		= (200),
	NK_N1_PTZ_CMD_PAN_RIGHT,
	NK_N1_PTZ_CMD_PAN_AUTO,
	NK_N1_PTZ_CMD_PAN_STOP_ALL,

	NK_N1_PTZ_CMD_ZOOM_IN			= (300),
	NK_N1_PTZ_CMD_ZOOM_OUT,

	NK_N1_PTZ_CMD_FOCUS_IN		= (400),
	NK_N1_PTZ_CMD_FOCUS_OUT,

	NK_N1_PTZ_CMD_IRIS_OPEN		= (500),
	NK_N1_PTZ_CMD_IRIS_CLOSE,
	NK_N1_PTZ_CMD_IRIS_ENLARGE,
	NK_N1_PTZ_CMD_IRIS_SHRINK,

	NK_N1_PTZ_CMD_SET_PRESET		= (1000),
	NK_N1_PTZ_CMD_GOTO_PRESET,
	NK_N1_PTZ_CMD_CLEAR_PRESET,
	NK_N1_PTZ_CMD_FILL_PRE_SEQ,		// Add preset point to cruise sequence
	NK_N1_PTZ_CMD_SET_SEQ_DWELL,		// Set dwell time for cruise point
	NK_N1_PTZ_CMD_RUN_SEQ,			// Start cruise
	NK_N1_PTZ_CMD_STOP_SEQ,			// Stop cruise
	NK_N1_PTZ_CMD_CLE_PRE_SEQ,		// Delete preset point from cruise speed
} NK_N1PTZCommand;

/**
 * Get the text information corresponding to the NK_N1PTZCommand enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1PTZCommand);

/**
 * Get the enum value corresponding to the NK_N1PTZCommand text information.
 */
extern DECLARE_NK_ENUM_UNMAP(N1PTZCommand);


/**
 * IRCut filter working type.
 */

//typedef NK_UInt32 NK_N1IRCutFilterMode;
//#define NK_N1_IRCUT_MODE_AUTO      NK_N1_UINT32_SYM('I', 'R', 'A', 0)
//#define NK_N1_IRCUT_MODE_DAYLIGHT  NK_N1_UINT32_SYM('I', 'R', 'D', 0)
//#define NK_N1_IRCUT_MODE_NIGHT     NK_N1_UINT32_SYM('I', 'R', 'N', 0)

typedef NK_N1PropEnum NK_N1IRCutFilterMode;

typedef enum Nk_N1IRCutMode
{
	 NK_N1_IRCUT_MODE_UNDEFINED   =   -1,
	 NK_N1_IRCUT_MODE_AUTO   =   NK_N1_UINT32_SYM('I', 'R', 'A', 0),
	 NK_N1_IRCUT_MODE_DAYLIGHT =  NK_N1_UINT32_SYM('I', 'R', 'D', 0),
	 NK_N1_IRCUT_MODE_NIGHT  =   NK_N1_UINT32_SYM('I', 'R', 'N', 0),
	 NK_N1_IRCUT_MODE_LIGHT  =   NK_N1_UINT32_SYM('I', 'R', 'L', 0),
	 NK_N1_IRCUT_MODE_SMART  =   NK_N1_UINT32_SYM('I', 'R', 'S', 0)
} NK_N1IRCutMode;
/**
 * Get the text information corresponding to the NK_N1IRCutMode enum value.
 */
extern DECLARE_NK_ENUM_MAP(N1IRCutMode);

/**
 * Get the enum value corresponding to the NK_N1IRCutMode text information.
 */
extern DECLARE_NK_ENUM_UNMAP(N1IRCutMode);


typedef enum Nk_N1Result
{
	/**
	 * Undefined error.
	 */
	NK_N1_ERR_UNDEF					= (-1),

	/**
	 * No error, operation successful.
	 */
	NK_N1_OK						= (0),
	NK_N1_ERR_NONE					= (NK_N1_OK),

	/**
	 * Parameter error related.
	 */
	NK_N1_ERR_INVALID_PARAM			= (101),	///< Parameter error.
	NK_N1_ERR_INVALID_CHANNEL_STREAM_ID,		///< Channel/Stream ID error.

	/**
	 * Device internal error related.
	 */
	NK_N1_ERR_DEVICE_BUSY			= (201),	///< Device busy, the function might be occupied.
	NK_N1_ERR_DEVICE_NOT_SUPPORT,				///< Device not supported.
	NK_N1_ERR_DEVICE_OUT_OF_MEMORY,
	NK_N1_ERR_DEVICE_OUT_OF_USER,

	/**
	 * Device operation error related.
	 */
	NK_N1_ERR_INVALID_OPERATE		= (301),	///< Invalid device operation.
	NK_N1_ERR_OPERATE_TIMEOUT,					///< Device operation timeout.

	/**
	 * Transmission packet error.
	 */
	NK_N1_ERR_INVALID_DATAGRAM		= (401),	///< Invalid transmission packet.


	/**
	 * Device upgrade.
	 */
	NK_N1_ERR_UPGRADE_FAILED		= (600),
	NK_N1_ERR_UPGRADE_NOT_SUPPORT,
	NK_N1_ERR_FIRMWARE_FILE_ERROR,
	NK_N1_ERR_UPGRADE_INTERRUPT,
	NK_N1_ERR_FIRMWARE_STORE_ERROR,
	NK_N1_ERR_FIRMWARE_VER_TOO_OLD,

	/**
	 * User authorization failed.
	 */
	NK_N1_ERR_NOT_AUTHORIZATED		= (1001),


	/**
	 * Two-way audio.
	 */
	NK_N1_ERR_2WAYTALK              = (1200),
	NK_N1_ERR_2WAYTALK_INVALID_ID,
	NK_N1_ERR_2WAYTALK_SEND_FAILED,
	NK_N1_ERR_2WAYTALK_BUSY,
	NK_N1_ERR_2WAYTALK_INVALID_PACK,


} NK_N1Error, NK_N1Result;

/**
 * Compatible versions < 1.4.0
 */
#define NK_N1Ret NK_N1Error


/**
 * Wi-Fi working mode.
 */
typedef enum Nk_N1EthWiFiMode {

	/**
	 * Wi-Fi not supported.
	 */
	NK_N1_ETH_WIFI_MODE_NA = (-1),

	/**
	 * Station mode.
	 */
	NK_N1_ETH_WIFI_MODE_STA = (0),

	/**
	 * Access point mode.
	 */
	NK_N1_ETH_WIFI_MODE_AP,

	/**
	 * Repeater mode.
	 */
	NK_N1_ETH_WIFI_MODE_REP,

} NK_N1EthWiFiMode;

#if 0
/**
 * Corresponding fields for Nk_N1EthWiFiMode enum.
 */
static NK_EnumStrMap
NK_ENUM_MAPPER(NK_N1EthWiFiMode)[] = {

		{	NK_N1_ETH_WIFI_MODE_NA,		"None",	},
		{	NK_N1_ETH_WIFI_MODE_STA,	"Station",	},
		{	NK_N1_ETH_WIFI_MODE_AP,		"Access Point",	},
		{	NK_N1_ETH_WIFI_MODE_REP,	"Repeater",	},

};

/**
 * Wi-Fi communication channel.
 */
typedef enum Nk_N1EthWiFiAccessChannel {

	NK_N1_ETH_WIFI_ACCESS_CH_AUTO = (0),
	NK_N1_ETH_WIFI_ACCESS_CH_1,
	NK_N1_ETH_WIFI_ACCESS_CH_2,
	NK_N1_ETH_WIFI_ACCESS_CH_3,
	NK_N1_ETH_WIFI_ACCESS_CH_4,
	NK_N1_ETH_WIFI_ACCESS_CH_5,
	NK_N1_ETH_WIFI_ACCESS_CH_6,
	NK_N1_ETH_WIFI_ACCESS_CH_7,
	NK_N1_ETH_WIFI_ACCESS_CH_8,
	NK_N1_ETH_WIFI_ACCESS_CH_9,
	NK_N1_ETH_WIFI_ACCESS_CH_10,
	NK_N1_ETH_WIFI_ACCESS_CH_11,
	NK_N1_ETH_WIFI_ACCESS_CH_12,
	NK_N1_ETH_WIFI_ACCESS_CH_13,
	NK_N1_ETH_WIFI_ACCESS_CH_14,

} NK_N1EthWiFiAccessChannel;

/**
 * Corresponding fields for NK_N1EthWiFiAccessChannel enum.
 */
static NK_EnumStrMap
NK_ENUM_MAPPER(NK_N1EthWiFiAccessChannel)[] = {

		{	NK_N1_ETH_WIFI_ACCESS_CH_AUTO,	"Auto",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_1,		"1",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_2,		"2",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_3,		"3",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_4,		"4",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_5,		"5",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_6,		"6",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_7,		"7",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_8,		"8",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_9,		"9",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_10,	"10",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_11,	"11",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_12,	"12",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_13,	"13",	},
		{	NK_N1_ETH_WIFI_ACCESS_CH_14,	"14",	},

};
#endif
typedef enum Nk_N1ImageSceneMode {
	NK_N1_IMG_SCENE_MODE_AUTO = (0),
	NK_N1_IMG_SCENE_MODE_INDOOR,
	NK_N1_IMG_SCENE_MODE_OUTDOOR,
} NK_N1ImageSceneMode;

typedef enum Nk_N1ImageExposureMode {
	/// Auto exposure mode.
	NK_N1_IMG_EXPO_MODE_AUTO = (0),
	/// Bright light exposure mode.
	NK_N1_IMG_EXPO_MODE_BRIGHT,
	/// Low light exposure mode.
	NK_N1_IMG_EXPO_MODE_DARK,
} NK_N1ImageExposureMode;

typedef enum Nk_N1ImageAutoWBMode {
	NK_N1_IMG_AWD_MODE_AUTO = (0),
	NK_N1_IMG_AWD_MODE_INDOOR,
	NK_N1_IMG_AWD_MODE_OUTDOOR,
} NK_N1ImageAutoWBMode;

typedef enum Nk_N1ImageBacklightCompensation {
	NK_N1_IMG_BL_COMP_AUTO = (0),
	NK_N1_IMG_BL_COMP_ALWAYS_ON,
	NK_N1_IMG_BL_COMP_ALWAYS_OFF,
} NK_N1ImageBacklightCompensation;

typedef enum Nk_N1LowlightMode{
		NK_N1_IMG_LL_MODE_OFF = (0),
		NK_N1_IMG_LL_MODE_ONLY_NIGHT,
		NK_N1_IMG_LL_MODE_DAY_AND_NIGHT,
		NK_N1_IMG_LL_MODE_AUTO,
	} NK_N1LowlightMode;
	
	
	typedef NK_UInt32 NK_N1MediaPackageType;
	
	#define NK_N1_MEDIA_PT_UNDEF    (NK_N1_UINT32_SYM(0, 0, 0, 0))
	#define NK_N1_MEDIA_PT_G711A    (NK_N1_UINT32_SYM('7', '1', '1', 'A'))
	#define NK_N1_MEDIA_PT_G711U    (NK_N1_UINT32_SYM('7', '1', '1', 'U'))
	#define NK_N1_MEDIA_PT_SPEEX    (NK_N1_UINT32_SYM('S', 'P', 'X', 0))
	#define NK_N1_MEDIA_PT_AAC      (NK_N1_UINT32_SYM('A', 'A', 'C', 0))
	#define NK_N1_MEDIA_PT_JPEG     (NK_N1_UINT32_SYM('J', 'P', 'E', 'G'))
	#define NK_N1_MEDIA_PT_H264     (NK_N1_UINT32_SYM('H', '2', '6', '4'))
	#define NK_N1_MEDIA_PT_H265     (NK_N1_UINT32_SYM('H', '2', '6', '5'))
	#define NK_N1_MEDIA_PT_HEVC     (NK_N1_MEDIA_PT_H265)
	
	/**
	 * Media package data definition.
	 */
	typedef struct Nk_N1MediaPackage {
	
		/// Media package type.
		NK_N1MediaPackageType type;
	
		/// Timestamp of the message, in us.
		NK_Size64 timestamp;
	
		/// Type attributes.
		union {
	
			struct {
	
				/// Video size.
				NK_Size width, height;
	
				/// Frame rate
				NK_Size framePS;
	
			} Video;
	
			struct {
	
				NK_Size sampleRate; ///< 8000, 11025, 16000, 22050, 24000...
				NK_Size sampleBitWidth; ///< Sample bit width, values 8/16/24/32.
				NK_Size samplePerPackage;
				NK_Size track;
	
			} Audio;
		};
	} NK_N1MediaPackage;
	
	
	/**
	 * Type of N1 live data carrier.
	 */
	typedef enum Nk_N1DataPayload
	{
		NK_N1_DATA_PT_UNDEF				= (-1),
	
		/**
		 * G.711 A-Law audio payload data.
		 */
		NK_N1_DATA_PT_G711A				= (8),
	
		/**
		 * Fisheye calibration.
		 */
		NK_N1_DATA_PT_FIXPARAM				= (103),
		/**
		 * AAC audio payload data.
		 */
		NK_N1_DATA_PT_AAC				= (104),
		/**
		 * JPEG Image.
		 */
		NK_N1_DATA_PT_JPEG			 	= (26),
	
		/**
		 * H.264 Nal-Unit video payload data.
		 */
		NK_N1_DATA_PT_H264_NALUS		= (96),
	
		/**
		 * HEVC Nal-Unit video payload data.
		 */
		NK_N1_DATA_PT_HEVC_NALUS		= (97),
	
		NK_N1_DATA_PT_CUSTOM			= (100),
	
		NK_N1_DATA_PT_CUSTOM2			= (102),
	
	} NK_N1DataPayload;
	
	/**
	 * Type of N1 live video data frame.
	 */
	typedef enum Nk_N1VideoFrameType
	{
		NK_N1_DATA_FRAME_UNUSE = 0,
	    NK_N1_DATA_FRAME_BASE_IDRSLICE = 1,                              //the Idr frame at Base layer
	    NK_N1_DATA_FRAME_BASE_PSLICE_REFTOIDR,                           //the P frame at Base layer,the P frame at Base layer, referenced by other frames at Base layer and reference to Idr frame
	    NK_N1_DATA_FRAME_BASE_PSLICE_REFBYBASE,                          //the P frame at Base layer, referenced by other frames at Base layer
	    NK_N1_DATA_FRAME_BASE_PSLICE_REFBYENHANCE,                       //the P frame at Base layer, referenced by other frames at Enhance layer
	    NK_N1_DATA_FRAME_ENHANCE_PSLICE_REFBYENHANCE,                    //the P frame at Enhance layer, referenced by other frames at Enhance layer
	    NK_N1_DATA_FRAME_ENHANCE_PSLICE_NOTFORREF,                       //the P frame at Enhance layer ,not referenced
	    NK_N1_DATA_FRAME_ENHANCE_PSLICE_BUTT
	} NK_N1VideoFrameType;
	
	/**
	 * Live session context data structure,
	 * This data structure is initialized when calling @ref NkN1Utils_InitLiveSession,
	 * and during session initialization.
	 *
	 */
	typedef struct Nk_N1LiveSession
	{
		/**
		 * Real-time media channel ID, starting from 0.
		 */
		NK_UInt32 channel_id;
	
		/**
		 * Stream ID under real-time media channel ID, starting from 0.
		 */
		NK_UInt32 stream_id;
	
		/**
		 * Unique session ID, generated internally by the module.
		 */
		NK_UInt32 session_id;
	
		/**
		 * Data frame sequence counter.
		 * Increments with each interface call.
		 */
		NK_UInt32 sequence;
	
		struct
		{
			NK_N1DataPayload payload_type; ///< Media type, see @ref NK_N1_LIVE_SESS_PT_* .
			NK_Size width, heigth; ///< Video width and height.
	
		} Video;
	
		struct
		{
			NK_N1DataPayload payload_type; ///< Media type, see @ref NK_N1_LIVE_SESS_PT_* .
			NK_UInt32 sample_rate; ///< Sample rate, number of audio samples per second.
			NK_UInt32 sample_bitwidth; ///< Sample bit width, can be 8, 16, 24, 32.
			NK_Boolean stereo; ///< Mono/Stereo flag, False for mono, True for stereo.
	
		} Audio;
	
		/**
		 * User session handle.
		 * The caller can use this handle to preserve context for the current session.
		 *
		 */
		NK_PVoid user_session;
	
		/**
		 * Reserved data area within the module.
		 */
		NK_Byte reserved[1024 * 2];
	
		/// @brief Bandwidth of the current session, read-only.
		NK_Size bandwidth;
	
		/// @brief Traffic statistics, read-only.
		NK_Size bytesum;
	
		/// @brief Traffic statistics duration, read-only.
		NK_Size timesum;
	
	} NK_N1LiveSession;
	
	
	/**
	 * @brief
	 *  N1 device capabilities set.
	 *
	 * @details
	 *
	 */
	typedef struct Nk_N1DeviceCapabilities {
	
	
		/// Device hardware code, used as a unique identifier for firmware upgrades,
		/// Default is 0.
		NK_UInt32 hwCode;
	
		/// Device version number, recommended format x.x.x,
		/// Default is SDK version number.
		NK_Char swVersion[32];
	
		/// Device name, returned as the name when the device is discovered,
		/// Default is HD IPCAM.
		NK_Char name[64];
	
		/// Device supports wired network RJ45 interface,
		/// Default is True.
		NK_Boolean supportRJ45;
	
		/// Device supports wireless station mode; set this flag for devices supporting WiFi station mode,
		/// Default is False.
		NK_Boolean supportWiFiStation;
	
		/// Device supports wireless access point mode; set this flag for devices supporting WiFi AP mode.
		/// Default is False.
		NK_Boolean supportWiFiAP;
	
		/// Device supports simultaneous wireless station and access point modes; set this flag.
		/// When this flag is set, @ref supportWiFiStation and @ref supportWiFiAP are ignored,
		/// Default is False.
		NK_Boolean supportWiFiRepeater;
	
		/// Maximum channel number, up to 128 channels,
		/// Default is 1.
		NK_Size maxMediaChannel;
	
		/// Channel capability description.
		struct {
			/// Maximum streams per channel,
			/// Default is 2.
			NK_Size maxStream;
	
			struct {
	
				/// Maximum concurrent access per stream,
				/// Default is 4.
				NK_Size maxOnCommand;
	
			} Stream[NK_N1_DEV_MAX_MEDIA_CH_STREAM];
	
		} MediaChannel[NK_N1_DEV_MAX_MEDIA_CH];
	
		/// Maximum TF card support,
		/// Default is 1.
		NK_Size maxTFCard;
	
		/// Maximum hard disk support,
		/// Default is 0.
		NK_Size maxHardDiskDriver;
	
	} NK_N1DeviceCapabilities;
	
	
	
	/**
	 * N1 data frame data structure.
	 * Consideration for caching frame data.
	 *
	 *
	 */
	typedef struct Nk_N1DataFrame
	{
		NK_Size n_vectors; ///< Number of valid vectors.
	
		struct
		{
			NK_PVoid raw; ///<
			NK_Size len;
	
		} Vectors[1024];
	
	} NK_N1DataFrame;
	
	
	///
	/// @brief
	///  N1 video encoder data structure definition.
	///
	typedef struct Nk_N1VideoEncoder {
	
		/// Encoder name, used to distinguish specific data structures.
		NK_N1PropEnum Codec;
	
		union {
			struct {
	
				/// Encoder configuration
				NK_N1PropEnum H264Profile;
	
				/// Encoding resolution.
				NK_N1PropEnum Resolution;
	
				/// Bit rate control mode.
				NK_N1PropEnum BitRateCtrlMode;
	
				/// Encoding bit rate (unit: kbps, kilobits per second).
				NK_N1PropInteger BitRate;
	
				/// Encoding frame rate (unit: fps, frames per second).
				NK_N1PropInteger FrameRate;
	
				/// Key frame interval (unit: frames, frames).
				NK_N1PropInteger KeyFrameInterval;
	
			} H264, H265, HEVC;
		};
	
	} NK_N1VideoEncoder;
	
	/**
	 * @brief
	 *  Network card configuration information.
	 */
	typedef struct Nk_N1EthConfig {
	
		/**
		 *
		 * +-------------+------------+------------+------------+------------+------------+
		 * |             | ESSID      | PSK        | EnableDHCP | HwAddr     | { IPv4 }   |
		 * +-------------+------------+------------+------------+------------+------------+
		 * | NetWired    | n          | n          | y          | y          | y          |
		 * +-------------+------------+------------+------------+------------+------------+
		 * | NetWiFi     | y          | y          | y          | y          | y          |
		 * +-------------+------------+------------+------------+------------+------------+
		 *
		 */
	
		/**
		 * ESSID corresponding to the connected wireless access point / NVR.
		 */
		NK_N1PropString ESSID;
	
		/**
		 * When classify is NK_N1_LAN_SETUP_WIFIAP,
		 * it represents the access point's password;
		 * when classify is NK_N1_LAN_SETUP_WIFISTA or NK_N1_LAN_SETUP_WIFINVR,
		 * it represents the password for the ESSID of the connected wireless access point / NVR.
		 */
		NK_N1PropString PSK;
	
		/**
		 * When classify is NK_N1_LAN_SETUP_WIFIAP,
		 * it indicates whether the local DHCP service is enabled;
		 * when classify is NK_N1_LAN_SETUP_WIFISTA,
		 * it indicates whether to use the wireless access point's DHCP service to obtain an address;
		 * when classify is NK_N1_LAN_SETUP_WIFINVR,
		 * this value is always False.
		 */
		NK_N1PropBoolean EnableDHCP;
	
		/**
		 * Physical network card address.
		 */
		NK_N1PropHwAddr HwAddr;
	
		/**
		 * IP address configuration.
		 */
		NK_N1PropIPv4 IPAddress, Netmask, Gateway, DomainNameServer;
	
		/**
		 * Wi-Fi working mode, corresponding to @ref Nk_N1EthWiFiMode type.
		 */
		NK_N1PropEnum WiFiMode;
	
		/**
		 * Wi-Fi connection parameters.
		 * Each object contains data (e.g., @ref _Station) and a data pointer (e.g., @ref Station); it is valid when the pointer points to the data.
		 * The data structure describes the essid and password needed for Wi-Fi connection, corresponding to @ref essid and @ref passphrase parameters.
		 * In repeater mode, the client sets two station configurations, @ref Station and @ref StationAlternative; which one to use depends on the device strategy.
		 * For station mode, @ref Station::essid and @ref Station::passphrase represent the name and password of the connected access point.
		 * For access point mode, @ref AccessPoint::essid and @ref AccessPoint::passphrase represent the broadcast name and password of the access point itself.
		 */
		struct {
	
			/**
			 * Name of the connected network access point.
			 */
			NK_Char essid[32];
	
			/**
			 * Password corresponding to the network @ref essid.
			 */
			NK_Char passphrase[32];
	
			/**
			 * Hidden SSID flag, valid only in access point and repeater modes.
			 */
			NK_Boolean hidden;
	
			/**
			 * Access point open channel, corresponding to @ref NK_N1EthWiFiAccessChannel,
			 * valid only in access point and repeater modes.
			 */
			NK_N1PropEnum AccessChannel;
	
		} _Station, _StationAlternative, _AccessPoint, *Station, *StationAlternative, *AccessPoint;
	
		/**
		 * Read-only, valid only in repeater mode.
		 */
		NK_Int connection;
	    /** 
	     * Rate in station or repeater mode
	     */
		NK_Int stationsignal;
		/**
		 * Connection information.
		 */
		struct {
	
			/**
			 * Physical address of the connected device.
			 */
			NK_N1PropHwAddr BssID;
	
			/**
			 * Connection rate.
			 */
			NK_N1PropInteger Rate;
	
		} Connection[8];
	 
		/// Temporary modification flag, reserved.
		NK_Boolean temporary;
	
	} NK_N1EthConfig;
	
	
	typedef enum Nk_N1LanSetupClassify
	{
	
		NK_N1_LAN_SETUP_UNDEF = (-1),
	
		/**
		 * Device information.
		 */
		NK_N1_LAN_SETUP_INFO,
	
		/**
		 * LAN time configuration, corresponding to NK_N1Lansetup::Time.
		 */
		NK_N1_LAN_SETUP_TIME,
	
		/**
		 * LAN infrared filter configuration, corresponding to NK_N1Lansetup::IRCutFilter.
		 */
		NK_N1_LAN_SETUP_IRCUT,
	
		/**
		 * LAN video image configuration, corresponding to NK_N1Lansetup::VideoImage.
		 */
		NK_N1_LAN_SETUP_VIMG,
	
		/**
		 * LAN video encoder configuration, corresponding to NK_N1Lansetup::VideoEncoder.
		 */
		NK_N1_LAN_SETUP_VENC,
	
		/**
		 * LAN PTZ control configuration, corresponding to NK_N1Lansetup::PanTiltZoom.
		 */
		NK_N1_LAN_SETUP_PTZ,
	
		NK_N1_LAN_SETUP_NET_WIRED,
		NK_N1_LAN_SETUP_NET_WIFI,
	
		/**
		 * DNS configuration.
		 */
		NK_N1_LAN_SETUP_DNS,
	
		/**
		 * P2P configuration.
		 */
		NK_N1_LAN_SETUP_P2P,
	
		/**
		 * Factory reset configuration.
		 */
		NK_N1_LAN_SETUP_FACTROY_RESET,
	
		/**
		 * RTSP configuration.
		 */
		NK_N1_LAN_SETUP_RTSP,
	
		/**
		 * Video OSD configuration.
		 */
		NK_N1_LAN_SETUP_VOSD,
	
		/**
		 * Video privacy mask configuration.
		 */
		NK_N1_LAN_SETUP_VPMSK,
		NK_N1_LAN_SETUP_HICONN,
	
		/**
		 * Onvif protocol configuration.
		 */
		NK_N1_LAN_SETUP_ONVIF,

	/// Time server synchronization settings.
	NK_N1_LAN_SETUP_NTP = 0x1021,
	
	/// Calendar usage settings.
	NK_N1_LAN_SETUP_CALENDAR = 0x1060,
	
	/// Advanced video image settings, corresponding to NK_N1LanSetup::VideoImageAdvanced.
	NK_N1_LAN_SETUP_VIM_ADV = 0x4010,

	/// Audio input/output configuration.
	NK_N1_LAN_SETUP_AIO = 0x6010,

	/// Audio encoding configuration, corresponding to NK_N1LanSetup::AudioEncoder.
	NK_N1_LAN_SETUP_AENC = 0x6040,


} NK_N1LanSetupClassify;


/**
 * N1 protocol LAN configuration related data structure.
 */
typedef struct Nk_N1LanSetup
{
	/**
	 * Configuration classification.
	 */
	NK_N1LanSetupClassify classify;

	/**
	 * Channel number, involved in multi-channel configurations.
	 */
	NK_Int channel_id;

	/**
	 * Stream number, involved in multi-stream configurations under a channel.
	 */
	NK_Int stream_id;

	/**
	 * Reference image width and height (unit: pixels).
	 *
	 */
	NK_Size ref_width, ref_height;

	union {

		/**
		 * Device information.
		 * Valid when classify equals NK_N1_LAN_SETUP_INFO.
		 */
		struct {

			/**
			 * Device's cloud ID number.
			 */
			NK_Char cloud_id[32];

			/**
			 * Device model definition.
			 */
			NK_Char model[32];

			/**
			 * Device version number.
			 */
			NK_Char version[32];

			/**
			 * Definition of device live channels, min 1, max 256.
			 */
			NK_Size live_channels;

			/**
			 * Attributes for each live channel, the effective number corresponds to @ref live_channels.
			 */
			struct {
				/**
				 * Number of streams for each live channel, min 1, max 8.
				 */
				NK_Size stream_channels;

			} LiveChannels[128];

			/**
			 * Device hardware code, related to upgrade policy.
			 */
			NK_UInt32 hardware_code;

			/**
			 * Manufacturer information.
			 */
			NK_Char manufacturer[32];

		} Info;

		/**
		 * Device time configuration related data structure.
		 * Valid when classify equals NK_N1_LAN_SETUP_TIME.
		 */
		struct 	{

			/**
			 * UTC time (Greenwich mean time timestamp relative to Jan 1, 1970, 00:00:00).
			 */
			NK_UTC1970 utc;

			/**
			 * Time zone.
			 */
			NK_TimeZone gmt;

			/**
			 * Daylight Saving Time (DST) usage flag.
			 */
			NK_Boolean dst;

		} Time;

		struct {
			NK_Boolean jalaali;
		} Calendar;

		struct {
			NK_Boolean enabled;
			NK_Char domain[64];
		} NTP;

		/**
		 * Infrared filter configuration related.
		 */
		struct {

			/**
			 * See @ref NK_N1IRCutFilterMode for details.
			 */
			NK_N1PropEnum Mode;

			/**
			 * Duration for judging conversion between day and night modes (unit: seconds).
			 */
			NK_N1PropInteger DayToNightFilterTime, NightToDayFilterTime;

		} IRCutFilter;

		/**
		 * Video image configuration related data structure.
		 * Valid when classify equals NK_N1_LAN_SETUP_VIMG.
		 */
		struct 	NK_VideoImage{

			/**
			 * Video image input frequency.
			 */
			NK_N1PropInteger PowerLineFrequenceMode;

			/**
			 * Video image input resolution.
			 */
			NK_N1PropEnum CaptureResolution;

			/**
			 * Video image input frame rate.
			 */
			NK_N1PropInteger CaptureFrameRate;

			/**
			 * Input image color adjustment.
			 * Related to device sensor or ISP processor.
			 */
			NK_N1PropInteger BrightnessLevel, ContrastLevel, SharpnessLevel, SaturationLevel, HueLevel;

			/**
			 * Horizontal/Vertical flip and mirror settings flag.
			 */
			NK_N1PropBoolean Flip, Mirror;

			/**
			 * Video image title.
			 */
			struct {

				/**
				 * Title display flag.
				 */
				NK_N1PropBoolean Show;

				/**
				 * Title text encoding.
				 */
				NK_N1PropEnum TextEncoding;

				/**
				 * Title text.
				 */
				NK_N1PropString Text;

			} Title;

			/**
			 * Video image motion detection.
			 */
			struct NK_MotionDetection{
				/**
				 * Enable motion detection flag.
				 */
				NK_N1PropBoolean Enabled;

				/**
				 * Detection sensitivity.
				 */
				NK_N1PropInteger SensitivityLevel;

				/**
				 * Detection area mask.
				 * Configuring video motion detection requires a detection mask in addition to sensitivity.
				 * When all active areas in the mask are True, it indicates full-video motion detection.
				 * Otherwise, it detects motion only in the active mask areas. The maximum mask granularity is 32x24; if the device implementation uses a larger granularity, adaptation is required.
				 *
				 */
				struct {
					/**
					 * Less than or equal to 32x24.
					 */
					NK_Size width, height;
					NK_Byte matrix[24][32];
				} Mask;

			} MotionDetection;

			/**
			 * Image color style; users can predefine multiple styles for selection.
			 */
			NK_N1PropInteger ColorStyle;

		} VideoImage;

		///
		/// Advanced image settings interface, corresponding to NK_N1_LAN_SETUP_VIM_ADV configuration options.
		///
		struct {

			NK_N1PropEnum SceneMode;
			NK_N1PropInteger Denoise3D;
			NK_N1PropInteger DigitalWDR;
			/// Exposure mode.
			NK_N1PropEnum ExposureMode;
			/// Auto White Balance (AWB) mode.
			NK_N1PropEnum AutoWBMode;
			/// Manual sharpening.
			NK_N1PropInteger ManualSharpen;
			// Manual sharpening switch
			NK_N1PropBoolean ManualSharpenSwitch;
			/// Backlight compensation.
			NK_N1PropEnum BacklightCompensation;
			///Lowlight
			NK_N1PropEnum LowlightMode;
		} VideoImageAdvanced;

		struct {

			NK_N1PropInteger SampleRate;
			NK_N1PropInteger SampleBitWidth;
			NK_N1PropInteger InputVolume;
			NK_N1PropInteger OutputVolume;
			NK_N1PropEnum InputMode;

		} AudioIO;

		struct {
			NK_Boolean enabled;
			NK_N1PropEnum Encodec;
		} AudioEncoder;

		/**
		 * Video encoding configuration related data structure.
		 */
		NK_N1VideoEncoder VideoEncoder;

		/**
		 * Video privacy mask configuration related data structure.
		 */
		struct {

            NK_N1PropBoolean Enabled;

			/**
			 * Mask area, related to @ref_width and @ref_height parameters.
			 */
			struct {

				/**
                 * Enable flag.
                 */
				NK_N1PropBoolean Enabled;

                NK_Int Color;

				NK_DFloat x,y,width,height;

			} Mask[4];

		} VideoPrivacyMask;

		/**
		 * Video editing layer configuration.
		 */
		struct {

			/**
			 * Display mode.
			 */
			NK_N1PropEnum Method;

			struct {

				/**
				 * Display flag.
				 */
				NK_N1PropBoolean Enabled;

				/**
				 * Display position.
				 */
				NK_Size x, y;

				/**
				 * Text encoding flag.
				 * True for UTF-8 encoding of @ref Text, False for GB2312.
				 */
				NK_Boolean textUTF8;

				/**
				 * Input text.
				 */
				NK_N1PropString Text;

			} Title;

			struct {

				/**
				 * Display flag.
				 */
				NK_N1PropBoolean Enabled;

				/**
				 * Display position.
				 */
				NK_Size x, y;

				/**
				 * Display weekday flag.
				 */
				NK_N1PropBoolean DisplayWeekday;

				/**
				 * Display time in 12-hour format flag.
				 */
				NK_N1PropBoolean TimeFormt12HRs;

				/**
				 * Date display format.
				 */
				NK_N1PropEnum DateFormat;

			} Time;

		} VideoOnScreenDisplay;

		/**
		 * PTZ control configuration related data structure.
		 * Valid when classify equals NK_N1_LAN_SETUP_PTZ.
		 */
		struct {

			NK_N1PTZCommand command;

			/**
			 * Single-step execution flag. For NK_N1_LAN_SETUP_PTZ_CMD_TILT_* or NK_N1_LAN_SETUP_PTZ_CMD_PAN_* commands,
			 * the device stops automatically after one step when this is set.
			 * Otherwise, the client must send NK_N1_LAN_SETUP_PTZ_CMD_STOP to stop the motion.
			 *
			 */
			NK_Boolean step;

			/**
			 * Preset position number. When the command is NK_N1_LAN_SETUP_PTZ_CMD_*_PRESET, it indicates the target preset.
			 */
			NK_Integer preset_position;

			/**
			 * PTZ motion speed.
			 */
			NK_N1PropInteger Speed;

		} PanTiltZoom;


		/**
		 * Wireless NVR connection configuration.
		 * Configures the wireless network card for device wireless connection based on NVR requests.
		 */
		NK_N1EthConfig NetWired, NetWiFi;

		/**
		 * DNS address configuration.
		 */
		struct {

			/**
			 * Preferred DNS address.
			 */
			NK_N1PropIPv4 Preferred;

			/**
			 * Alternative DNS address.
			 */
			NK_N1PropIPv4 Alternative;

		} DNS;


		/**
		 * P2P configuration.
		 */
		struct {

			NK_N1PropBoolean Enabled;

		} PtoP;

		/**
		 * Onvif configuration.
		 */
		struct {

			/**
			 * IP auto-adaptation enable flag.
			 * Recorded in the configuration file.
			 */
			NK_N1PropBoolean IPAutoAdaption;

			/**
			 * Temporary configuration variable, not saved to configuration.
			 * This flag is not strictly tied to @ref IPAutoAdaption; it indicates the module's desire to temporarily enable/disable auto-adaptation.
			 * In upper-layer implementations, IP auto-adaptation is active only when both @ref IPAutoAdaption and @ref ipAutoAdaptionActived are enabled.
			 *
			 */
			NK_Boolean ipAutoAdaptionActived;


		} Onvif;
	};

} NK_N1LanSetup;


/**
 * Print NK_N1LanSetup data structure to terminal for debugging.
 *
 */
static inline NK_Void
NK_N1_LAN_SETUP_DUMP(NK_N1LanSetup *__LanSetup)
{
	NK_Int i = 0, ii = 0;
	NK_TermTable Table;
	NK_Char text[64];
	NK_CHECK_POINT();
	NK_TermTbl_BeginDraw(&Table, "Lan Setup Data Field", 80, 4);

	if (NK_N1_LAN_SETUP_TIME == (__LanSetup)->classify) {
		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Time");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "UTC", "%u", (__LanSetup)->Time.utc);
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Time Zone", "%d", (__LanSetup)->Time.gmt);
		NK_TermTbl_PutKeyValue(&Table, NK_True, "DST", "%s", (__LanSetup)->Time.dst ? "Enabled" : "Disabled");
	} else if (NK_N1_LAN_SETUP_VIMG == (__LanSetup)->classify) {
		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Video Image");
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Channel", "%d", (__LanSetup)->channel_id);
		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Title");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Show", "%s", (__LanSetup)->VideoImage.Title.Show.val ? "Yes" : "No");
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Text", "%s", (__LanSetup)->VideoImage.Title.Text.val);
		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Color");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Hue Level", "%d [%d, %d]",
			(__LanSetup)->VideoImage.HueLevel.val, (__LanSetup)->VideoImage.HueLevel.min, (__LanSetup)->VideoImage.HueLevel.max);
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Sharpness Level", "%d [%d, %d]",
			(__LanSetup)->VideoImage.SharpnessLevel.val, (__LanSetup)->VideoImage.SharpnessLevel.min, (__LanSetup)->VideoImage.SharpnessLevel.max);
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Contrast Level", "%d [%d, %d]",
			(__LanSetup)->VideoImage.ContrastLevel.val, (__LanSetup)->VideoImage.ContrastLevel.min, (__LanSetup)->VideoImage.ContrastLevel.max);
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Brightness Level", "%d [%d, %d]",
			(__LanSetup)->VideoImage.BrightnessLevel.val, (__LanSetup)->VideoImage.BrightnessLevel.min, (__LanSetup)->VideoImage.BrightnessLevel.max);
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Saturation Level", "%d [%d, %d]",
			(__LanSetup)->VideoImage.SaturationLevel.val, (__LanSetup)->VideoImage.SaturationLevel.min, (__LanSetup)->VideoImage.SaturationLevel.max);
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Flip / Mirror", "%s / %s",
			(__LanSetup)->VideoImage.Flip.val ? "Yes" : "No", (__LanSetup)->VideoImage.Mirror.val ? "Yes" : "No");
		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Motion Detection");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Enabled", "%s", (__LanSetup)->VideoImage.MotionDetection.Enabled.val ? "Yes" : "No");
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Sensitivity Level", "%d [%d, %d]",
			(__LanSetup)->VideoImage.MotionDetection.SensitivityLevel.val,
			(__LanSetup)->VideoImage.MotionDetection.SensitivityLevel.min,
			(__LanSetup)->VideoImage.MotionDetection.SensitivityLevel.max);
		NK_TermTbl_PutText(&Table, NK_False, "%s (%u x %u)", "Mask",
				(__LanSetup)->VideoImage.MotionDetection.Mask.width, (__LanSetup)->VideoImage.MotionDetection.Mask.height);
		for (i = 0; i < (NK_Int)((__LanSetup)->VideoImage.MotionDetection.Mask.height); ++i) {
			NK_Char mask[80];
			NK_BZERO(mask, sizeof(mask));
			for (ii = 0; ii < (NK_Int)((__LanSetup)->VideoImage.MotionDetection.Mask.width); ++ii) {
				mask[ii * 2] = mask[ii * 2 + 1] = (__LanSetup)->VideoImage.MotionDetection.Mask.matrix[i][ii] ? 'O' : '.';
			}
			NK_TermTbl_PutText(&Table, NK_False, "%s", mask);
		}

	} else if (NK_N1_LAN_SETUP_VENC == (__LanSetup)->classify) {
		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Video Encoder");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Channel", "%d", (__LanSetup)->channel_id);
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Stream", "%d", (__LanSetup)->stream_id);
		if (NK_N1_VENC_CODEC_H264 == (__LanSetup)->VideoEncoder.Codec.val) {
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Resolution", "%d", (__LanSetup)->VideoEncoder.H264.Resolution.val);
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Bit Rate", "%d", (__LanSetup)->VideoEncoder.H264.BitRate.val);
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Frame Rate", "%d", (__LanSetup)->VideoEncoder.H264.FrameRate.val);
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Key Frame Interval", "%d", (__LanSetup)->VideoEncoder.H264.KeyFrameInterval.val);
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Bit Rate Control Mode", "%s", NK_ENUM_MAP(N1BitRateCtrlMode, (NK_N1BitRateCtrlMode)((__LanSetup)->VideoEncoder.H264.BitRateCtrlMode.val)));
		}
	} else if (NK_N1_LAN_SETUP_VPMSK == (__LanSetup)->classify) {

		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Video Privacy Mask");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Channel", "%d", (__LanSetup)->channel_id);
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Enabled", "%s", (__LanSetup)->VideoPrivacyMask.Enabled.val ? "Yes" : "No");
		for (i = 0; i < sizeof((__LanSetup)->VideoPrivacyMask.Mask) / sizeof((__LanSetup)->VideoPrivacyMask.Mask[0]); ++i) {
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Mask", "%u,%u,%u,%u",
					(NK_UInt32)((__LanSetup)->VideoPrivacyMask.Mask[i].x),
					(NK_UInt32)((__LanSetup)->VideoPrivacyMask.Mask[i].y),
					(NK_UInt32)((__LanSetup)->VideoPrivacyMask.Mask[i].width),
					(NK_UInt32)((__LanSetup)->VideoPrivacyMask.Mask[i].height));
		}
	} else if (NK_N1_LAN_SETUP_VOSD == (__LanSetup)->classify) {

		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Video OSD");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Channel", "%d", (__LanSetup)->channel_id);
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Reference Size", "%ux%u", (NK_UInt32)((__LanSetup)->ref_width), (NK_UInt32)((__LanSetup)->ref_height));
		NK_TermTbl_PutText(&Table, NK_True, "Title");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Show", "%s", (__LanSetup)->VideoOnScreenDisplay.Title.Enabled.val ? "Yes" : "No");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Position", "(%u, %u)",
				(NK_UInt32)((__LanSetup)->VideoOnScreenDisplay.Title.x), (NK_UInt32)((__LanSetup)->VideoOnScreenDisplay.Title.y));
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Text", "%s", (__LanSetup)->VideoOnScreenDisplay.Title.Text.val);
		NK_TermTbl_PutText(&Table, NK_True, "Time");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Show", "%s", (__LanSetup)->VideoOnScreenDisplay.Time.Enabled.val ? "Yes" : "No");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Show Weekday", "%s", (__LanSetup)->VideoOnScreenDisplay.Time.DisplayWeekday.val ? "Yes" : "No");
//		NK_TermTbl_PutKeyInt32(&Table, NK_False, "Date Format", (__LanSetup)->VideoOnScreenDisplay.osdType);
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Time Format", "%s", (__LanSetup)->VideoOnScreenDisplay.Time.TimeFormt12HRs.val ? "12HRs" : "24HRs");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Position", "(%u, %u)",
				(NK_UInt32)((__LanSetup)->VideoOnScreenDisplay.Time.x),
				(NK_UInt32)((__LanSetup)->VideoOnScreenDisplay.Time.y));

	}else if (NK_N1_LAN_SETUP_PTZ == (__LanSetup)->classify) {

		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Pan Tilt Zoom");
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Channel", "%d", (__LanSetup)->channel_id);
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Command", NK_ENUM_MAP(N1PTZCommand, (__LanSetup)->PanTiltZoom.command));
		if (NK_N1_PTZ_CMD_SET_PRESET == (__LanSetup)->PanTiltZoom.command
				|| NK_N1_PTZ_CMD_GOTO_PRESET == (__LanSetup)->PanTiltZoom.command
				|| NK_N1_PTZ_CMD_CLEAR_PRESET == (__LanSetup)->PanTiltZoom.command) {
			NK_TermTbl_PutKeyValue(&Table, NK_False, "Preset Position", "%d", (__LanSetup)->PanTiltZoom.preset_position);
		}
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Speed", "%d [%d, %d]", (__LanSetup)->PanTiltZoom.Speed.val,
				(__LanSetup)->PanTiltZoom.Speed.min, (__LanSetup)->PanTiltZoom.Speed.max);
	} else if (NK_N1_LAN_SETUP_NET_WIRED == (__LanSetup)->classify
			|| NK_N1_LAN_SETUP_NET_WIFI == (__LanSetup)->classify) {

		if (NK_N1_LAN_SETUP_NET_WIFI == (__LanSetup)->classify) {

			if ((NK_N1_ETH_WIFI_MODE_STA == (__LanSetup)->NetWiFi.WiFiMode.val
					|| NK_N1_ETH_WIFI_MODE_REP == (__LanSetup)->NetWiFi.WiFiMode.val)
					&& NK_Nil != (__LanSetup)->NetWiFi.Station) {
				NK_TermTbl_PutText(&Table, NK_True, "%s", "Wi-Fi Station");
				NK_TermTbl_PutKeyValue(&Table, NK_False, "EssID", "%s", (__LanSetup)->NetWiFi.Station->essid);
				NK_TermTbl_PutKeyValue(&Table, NK_True, "Passphrase", "%s", (__LanSetup)->NetWiFi.Station->passphrase);
			}

			if (NK_N1_ETH_WIFI_MODE_REP == (__LanSetup)->NetWiFi.WiFiMode.val
					&& NK_Nil != (__LanSetup)->NetWiFi.StationAlternative) {
				NK_TermTbl_PutText(&Table, NK_True, "%s", "Wi-Fi Station Alternative");
				NK_TermTbl_PutKeyValue(&Table, NK_False, "EssID", "%s", (__LanSetup)->NetWiFi.StationAlternative->essid);
				NK_TermTbl_PutKeyValue(&Table, NK_True, "Passphrase", "%s", (__LanSetup)->NetWiFi.StationAlternative->passphrase);
			}

			if ((NK_N1_ETH_WIFI_MODE_AP == (__LanSetup)->NetWiFi.WiFiMode.val
					|| NK_N1_ETH_WIFI_MODE_REP == (__LanSetup)->NetWiFi.WiFiMode.val)
					&& NK_Nil != (__LanSetup)->NetWiFi.AccessPoint) {
				NK_TermTbl_PutText(&Table, NK_True, "%s", "Wi-Fi Access Point");
				NK_TermTbl_PutKeyValue(&Table, NK_False, "EssID", "%s", (__LanSetup)->NetWiFi.AccessPoint->essid);
				NK_TermTbl_PutKeyValue(&Table, NK_True, "Passphrase", "%s", (__LanSetup)->NetWiFi.AccessPoint->passphrase);
			}

		} else {
			NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Wired Net");
		}
		NK_N1_PROP_HWADDR_STR(&(__LanSetup)->NetWiFi.HwAddr, text, sizeof(text));
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Hardware Address", "%s", text);
		NK_TermTbl_PutKeyValue(&Table, NK_False, "DHCP", "%s", (__LanSetup)->NetWiFi.EnableDHCP.val ? "Enabled" : "Disabled");
		NK_N1_PROP_IPV4_NTOA(&(__LanSetup)->NetWiFi.IPAddress, text, sizeof(text));
		NK_TermTbl_PutKeyValue(&Table, NK_False, "IP Address", "%s", text);
		NK_N1_PROP_IPV4_NTOA(&(__LanSetup)->NetWiFi.Netmask, text, sizeof(text));
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Netmask", "%s", text);
		NK_N1_PROP_IPV4_NTOA(&(__LanSetup)->NetWiFi.Gateway, text, sizeof(text));
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Gateway", "%s", text);
		NK_N1_PROP_IPV4_NTOA(&(__LanSetup)->NetWiFi.DomainNameServer, text, sizeof(text));
		NK_TermTbl_PutKeyValue(&Table, NK_False, "DNS", "%s", text);
	} else if (NK_N1_LAN_SETUP_DNS == (__LanSetup)->classify) {
		NK_N1_PROP_IPV4_NTOA(&(__LanSetup)->DNS.Preferred, text, sizeof(text));
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Preferred DNS", "%s", text);
		NK_N1_PROP_IPV4_NTOA(&(__LanSetup)->DNS.Alternative, text, sizeof(text));
		NK_TermTbl_PutKeyValue(&Table, NK_False, "Alternative DNS", "%s", text);
	} else if (NK_N1_LAN_SETUP_P2P == (__LanSetup)->classify) {
		NK_TermTbl_PutKeyValue(&Table, NK_False, "P2P", "%s", (__LanSetup)->PtoP.Enabled.val ? "Enabled" : "Disabled");
	} else if (NK_N1_LAN_SETUP_ONVIF == (__LanSetup)->classify) {
		NK_TermTbl_PutText(&Table, NK_True, "%-48s", "Onvif");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "IP Audio Adaption Enabled", "%s", (__LanSetup)->Onvif.IPAutoAdaption.val ? "Enabled" : "Disabled");
		NK_TermTbl_PutKeyValue(&Table, NK_False, "IP Audio Adaption Actived", "%s", (__LanSetup)->Onvif.ipAutoAdaptionActived ? "Actived" : "Deactived");
	}

	NK_TermTbl_EndDraw(&Table);
}

/**
 * N1 notification types.
 *
 */
typedef NK_Size Nk_N1NotificationType;
#define NK_N1_NOTF_NA                 (0)     ///< Invalid notification
#define NK_N1_NOTF_MOTION_DETECTED    (0x200) ///< Motion detection notification.

#define NK_N1_NOTF_VIDEO_LOSS         (0x400) ///< Video loss notification
#define NK_N1_NOTF_VIDEO_SHELTER      (0x402) ///< Video occlusion notification

#define NK_N1_NOTF_HDD_NOT_FOUND      (0x700)  ///< HDD not found notification
#define NK_N1_NOTF_HDD_FULL           (0x702)  ///< HDD full notification
#define NK_N1_NOTF_HDD_RECORD_ERR     (0x704)  ///< HDD recording exception notification

#define NK_N1_NOTF_REMOTE_KEYPAD      (0x10000) ///< Remote control related notification
#define NK_N1_NOTF_DOOR_MAGNETIC      (0x11000) ///< Door magnetic sensor related notification

#define NK_N1_NOTF_PIR_DETECTED       (0x12000) ///< PIR detection notification
#define NK_N1_NOTF_SMOKE_DETECTED     (0x13000) ///< Smoke detection notification
	

/**
 * N1 notification data structure.
 */
typedef struct Nk_N1Notification
{
	Nk_N1NotificationType type;

	union {

		/// Motion detection notification, valid when @ref NK_N1Notification::type is NK_N1_NOTF_MOTION_DETECTED.
		struct {
			NK_UInt32 reserved;
		} MotionDetected;

		/// PIR detection, valid when @ref NK_N1Notification::type is NK_N1_NOTF_PIR_DETECTED.
		struct {
			NK_UInt32 reserved;
		} PIRDetected;
	};

} NK_N1Notification;



/**
 * Wireless access point data structure definition.
 */
typedef struct Nk_WiFiHotSpot
{
	/**
	 * BSSID of the wireless access point.
	 */
	NK_Char bssid[32];
	/**
	 * Wireless access point communication channel, 0 for automatic.
	 */
	NK_Int channel;
	/**
	 * Wireless access point signal strength.
	 */
	NK_Int dBm, sdBm;
	/**
	 * Wireless access point age.
	 */
	NK_Int age;

	/**
	 * ESSID of the wireless access point.
	 */
	NK_Char essid[128];

	/**
	 * PSK of the wireless access point.
	 */
	NK_Char psk[32];

} NK_WiFiHotSpot;


/**
 * Print NK_WiFiHotSpot data structure.
 */
#define NK_N1_WIFI_HOTSPOT_DUMP(__HotSpot) \
	do{\
		NK_TermTable Table;\
		NK_TermTbl_BeginDraw(&Table, "N1 Hot Spot", 64, 4);\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "BSSID", "%s", (__HotSpot)->bssid);\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Channel", "%d", (__HotSpot)->channel);\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "dBm / SdBm", "%d / %d", (__HotSpot)->dBm, (__HotSpot)->sdBm);\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "Age", "%d", (__HotSpot)->age);\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "ESSID", "%s", (__HotSpot)->essid);\
		NK_TermTbl_PutKeyValue(&Table, NK_True, "PSK", "%s", (__HotSpot)->psk);\
		NK_TermTbl_EndDraw(&Table);\
	} while(0)


NK_CPP_EXTERN_END
#endif /* NK_N1_DEF_H_ */
