/**
 * N1 protocol property definition related data structures.
 */
#include <string.h>
#include <NkUtils/types.h>
#include <NkUtils/log.h>
#include <NkUtils/macro.h>


#ifndef NK_N1_PROPERTY_H_
#define NK_N1_PROPERTY_H_
NK_CPP_EXTERN_BEGIN

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
 * Check whether the property data structure is valid.
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
 * Mainly for debugging.
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
	 * In some cases, when the property value has an error, the default may serve as an adjustment reference.
	 */
	NK_Int32 def;

	/**
	 * Property min/max values.\n
	 * When both min and max are 0, the property value is unconstrained by min/max.\n
	 * Otherwise, both the value and the default must lie between min and max for the property to be valid.
	 */
	NK_Int min, max;

	/**
	 * Property option data structure.\n
	 * When Option is not Nil, @ref min and @ref max values are invalid.\n
	 * The value range is determined by the options.
	 */
	struct {
		NK_Size entries;
		NK_Int32 opt[NK_N1_PROP_OPT_MAX_ENT];
	} _Option, *Option;

} NK_N1PropInteger;

/**
 * Check whether the property data structure is valid.
 */
static inline NK_PChar
NK_N1_PROP_INT_CHECK(NK_N1PropInteger *(__Prop))
{
	NK_Int i = 0;

	if (!(__Prop)->Option) {
		/// Check whether the value and default are within range.
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
 * Mainly for debugging.
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
	/// Check whether the value is within the corresponding value domain.
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
 * Mainly for debugging.
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
 * Enum property is similar to integer property.\n
 * Unlike integer property, enum property has no min/max constraints but must have options.\n
 * Each enum option value must have a corresponding text string.
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
		 * Text corresponding to option values.
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
	/// Evaluate enum structure data value conditions.
	if (!(__Prop)->Option) {
		return "Option NULL";
	}else{
		if (!((__Prop)->Option->entries > 0
				&& (__Prop)->Option->entries < (sizeof((__Prop)->Option->opt)/sizeof((__Prop)->Option->opt[0])))) {
			return "Option Range Error.";
		}
		/// Traverse all enum options; each enum value should not have a null string mapping.
		for (i = 0; i < (NK_Int)(__Prop)->Option->entries; ++i) {
			if (!(__Prop)->Option->str[i]) {
				return "Option Text Error.";
			}
		}
		/// Traverse all enum options to ensure value is within options.
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
 * Mainly for debugging.
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
	/// Evaluate float data structure values.
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
 * Print NK_N1PropEnum data structure to terminal.
 * Mainly for debugging.
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
 * String property data structure.
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
	 * Maximum text length, distinct from @ref NK_N1_PROP_STR_MAX_LEN.
	 * Text is valid only if its length is within this limit.
	 */
	NK_Size max_len;

	/**
	 * See NK_N1PropInteger::Option.
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
				/** Must match data in options.*/\
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
	/// Check string value conditions and range.
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
 * Print NK_N1PropEnum data structure to terminal.
 * Mainly for debugging.
 */
static inline
NK_Void NK_N1_PROP_STRING_DUMP(NK_N1PropString *(__Prop), const NK_PChar val_name)
{
//	NK_Int i;
//	NK_TermTable Table;
//	NK_PChar err = NK_Nil;
//	NK_TermTbl_BeginDraw(&Table, !(val_name) ? "N1 Property String" : (val_name), 32, 4);
//	/// Check string data type value and range.
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
 * Set the address of the HW address property.
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
 * Convert text to HW address property.
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
 * Convert HW address property to text.
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
		return "Ivalid Address.";
	}
	return NK_Nil;
}

/**
 * Print NK_N1PropHwAddr data structure to terminal.
 * Mainly for debugging.
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
 * Set the address of the IPv4 property.
 */
#define NK_N1_PROP_IPV4_SET(__Prop, __ip0, __ip1, __ip2, __ip3) \
	do{\
		(__Prop)->val[0] = (NK_UInt32)(__ip0);\
		(__Prop)->val[1] = (NK_UInt32)(__ip1);\
		(__Prop)->val[2] = (NK_UInt32)(__ip2);\
		(__Prop)->val[3] = (NK_UInt32)(__ip3);\
	} while(0)

/**
 * Convert text to IPv4 property setting.
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
 * Convert IPv4 property to text.
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
		return "Ivalid Address.";
	}
	return NK_Nil;
}

/**
 * Print NK_N1PropIPv4 data structure to terminal.
 * Mainly for debugging.
 */
static inline NK_Void
NK_N1_PROP_IPV4_DUMP(NK_N1PropIPv4 *(__Prop), const NK_PChar val_name)
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
 * Mainly for debugging.
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
		}\
		if ((__Prop)->Option->entries >= NK_N1_PROP_OPT_MAX_ENT){\
		/* Option quota is full. */\
			break;\
		}\
		(__Prop)->Option->opt[(__Prop)->Option->entries++] = (__val);\
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



NK_CPP_EXTERN_END
#endif /* NK_N1_PROPERTY_H_ */
