/**
 * Logging singleton module.
 */

#include <NkUtils/types.h>

#if !defined(NK_UTILS_LOG_H_)
#define NK_UTILS_LOG_H_
NK_CPP_EXTERN_BEGIN

/**
 * @macro
 *  Maximum length of a single log line; lines exceeding this length are automatically truncated.
 */
#define NK_LOG_MAX_LINE_SZ (1024)

/**
 * @macro
 *  Log internal buffer capacity; when the internal buffer reaches this capacity, the onFlush event is triggered to clear the buffer.
 */
#define NK_LOG_CACHE_SZ    (1024 * 16)

/**
 * Log processing level.
 */
typedef enum Nk_LogLevel
{
	NK_LOG_ALL = (0),
	NK_LOG_LV_DEBUG,
	NK_LOG_LV_INFO,
	NK_LOG_LV_WARN,
	NK_LOG_LV_ERROR,
	NK_LOG_LV_ALERT,
	NK_LOG_NONE,

} NK_LogLevel;


/**
 * Log tag definition.
 */
#define NK_LOG_TAG(__level) \
	((NK_LOG_LV_ALERT == __level) ? "alert" : (\
			(NK_LOG_LV_ERROR == __level) ? "error" : (\
					(NK_LOG_LV_WARN == __level) ? "warn" : (\
							(NK_LOG_LV_INFO == __level) ? "info" : (\
									(NK_LOG_LV_DEBUG == __level) ? "debug" : "undef")))))


/**
 * Log singleton module handle.
 */
struct Nk_Log
{
	/**
	 * @brief
	 *  Set the log save level.\n
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @brief
	 *  After setting the level, only logs whose output level is less than or equal to the set level will be saved; all others are discarded.\n
	 *  The log save path is configured via @ref setLogPath.\n
	 *  For example, if the level is set to @ref NK_LOG_LV_NOTICE, only logs output via @ref error and @ref notice will be processed;\n
	 *  logs from @ref info and @ref debug will be automatically discarded.\n
	 *  The default level when not set is @ref NK_LOG_LV_ERROR.
	 *
	 */
	NK_Void
	(*setLogLevel)(NK_LogLevel level);

	/**
	 * @brief
	 *  Get the log save level.\n
	 *  This feature is disabled in version >= 33 for security reasons.
	 */
	NK_LogLevel
	(*getLogLevel)();

	/**
	 * @brief
	 *  Set the log console output level.\n
	 * @details
	 *  Only logs whose output level is less than or equal to the set level will be printed to the console; others are ignored.\n
	 *  The default level when not set is @ref NK_LOG_LV_INFO.
	 *
	 */
	NK_Void
	(*setTerminalLevel)(NK_LogLevel level);

	/**
	 * @brief
	 *  Get the log console output level.
	 */
	NK_LogLevel
	(*getTerminalLevel)();

	/**
	 * Alert log record.
	 */
	NK_Void
	(*alert)(const NK_Char* fmt, ...);

	/**
	 * Error log record.
	 */
	NK_Void
	(*error)(const NK_Char* fmt, ...);

	/**
	 * Warning log record.
	 */
	NK_Void
	(*warn)(const NK_Char* fmt, ...);

	/**
	 * Info log record.
	 */
	NK_Void
	(*info)(const NK_Char* fmt, ...);

	/**
	 * Debug log record.
	 */
	NK_Void
	(*debug)(const NK_Char* fmt, ...);

	/**
	 * @brief
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  The module maintains an internal log buffer; all logs below the Terminal Level are recorded into the buffer.\n
	 *  When the buffer is full, this interface is automatically triggered to flush it, and the @ref onFlush event is also triggered.\n
	 *  Users can handle the buffered log data in the @ref onFlush event according to their implementation.
	 */
	NK_Void
	(*flush)();

	/**
	 * @brief
	 *  Log buffer full flush event.\n
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  See @ref flush.\n
	 *  Note: NK_Log module interfaces must not be called within the event implementation, as doing so may cause a crash due to infinite program loop.
	 */
	NK_Void
	(*onFlush)(NK_PByte bytes, NK_Size len);

	/**
	 * @brief
	 *  Log output event.
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  This event is triggered whenever the user calls a valid log output interface.\n
	 *  Users can redirect the message to another destination (e.g., network address, file) as needed.\n
	 *  Leaving this event as Nil ignores it.
	 *
	 * @param classify [in]
	 *  The level of the current message; must be within valid levels.
	 * @param mesg [in]
	 *  Message content.
	 *
	 */
	NK_Void
	(*onTrace)(NK_LogLevel level, NK_PChar msg);

	/**
	 * @brief
	 *  Print module status.
	 */
	NK_Void
	(*dump)();

	/**
	 * @brief
	 *  Set the module log level.
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  Configures the output level for module @ref mod.\n
	 *  Unconfigured modules default to @ref NK_LOG_NONE level with no log output.\n
	 *  If the @ref mod name already exists in the log records, its level will be updated.\n
	 *  This interface is not thread-safe; it is recommended to call it globally during program initialization and avoid frequent calls during multi-threaded logging.
	 *
	 * @param mod [in]
	 *  Module name.
	 * @param level [in]
	 *  Log level.
	 *
	 * @return
	 *  Returns True on success, False on failure.
	 */
	NK_Boolean
	(*setModLevel)(const NK_PChar mod, NK_LogLevel level);

	/**
	 * @brief
	 *  Set the module log level.
	 *  This feature is disabled in version >= 33 for security reasons.
	 *
	 * @param mod [in]
	 *  Module name.
	 *
	 * @return
	 *  Returns the level for that module.
	 */
	NK_LogLevel
	(*getModLevel)(const NK_PChar mod);


	/**
	 * @brief
	 *  Output a log at @ref NK_LOG_LV_ALERT level.
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  Outputs for module @ref mod.\n
	 *  See @ref setModLevel() for setting module log levels.
	 */
	NK_Void
	(*alertV2)(const NK_PChar mod, const NK_Char* fmt, ...);

	/**
	 * @brief
	 *  Output a log at @ref NK_LOG_LV_ERROR level.
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  Outputs for module @ref mod.\n
	 *  See @ref setModLevel() for setting module log levels.
	 */
	NK_Void
	(*errorV2)(const NK_PChar mod, const NK_Char* fmt, ...);

	/**
	 * @brief
	 *  Output a log at @ref NK_LOG_LV_WARN level.
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  Outputs for module @ref mod.\n
	 *  See @ref setModLevel() for setting module log levels.
	 */
	NK_Void
	(*warnV2)(const NK_PChar mod, const NK_Char* fmt, ...);

	/**
	 * @brief
	 *  Output a log at @ref NK_LOG_LV_INFO level.
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  Outputs for module @ref mod.\n
	 *  See @ref setModLevel() for setting module log levels.
	 */
	NK_Void
	(*infoV2)(const NK_PChar mod, const NK_Char* fmt, ...);

	/**
	 * @brief
	 *  Output a log at @ref NK_LOG_LV_DEBUG level.
	 *  This feature is disabled in version >= 33 for security reasons.
	 * @details
	 *  Outputs for module @ref mod.\n
	 *  See @ref setModLevel() for setting module log levels.
	 */
	NK_Void
	(*debugV2)(const NK_PChar mod, const NK_Char* fmt, ...);

};

/**
 * Get the singleton handle.
 */
NK_API struct Nk_Log
*NK_Log();


/**
 * Terminal table output handle.
 * Stores the context during the table output process.
 */
typedef struct Nk_TermTable
{
	NK_Byte reserved[128];

} NK_TermTable;

/**
 * Begin drawing a terminal table.
 * Initializes the @ref Tbl handle.
 */
NK_API NK_Int
NK_TermTbl_BeginDraw(NK_TermTable *Tbl, NK_PChar title, NK_Size width, NK_Size padding);

/**
 * Append a plain text row to the terminal table.
 *
 * @param[in]			end_ln			Row-end flag; when True, a bottom border is appended after the text.
 * @param[in]			fmt				Text output format.
 *
 * @retval 0		Output succeeded.
 * @retval -1		Output failed.
 *
 */
NK_API NK_Int
NK_TermTbl_PutText(NK_TermTable *Tbl, NK_Boolean end_ln, NK_PChar fmt, ...);

/**
 * Append a "key - value" format row to the terminal table.
 *
 * @param[in]			end_ln			Row-end flag; when True, a bottom border is appended after the text.
 * @param[in]			key				Key text.
 * @param[in]			fmt				Value text output format.
 *
 * @retval 0		Output succeeded.
 * @retval -1		Output failed.
 *
 */
NK_API NK_Int
NK_TermTbl_PutKeyValue(NK_TermTable *Tbl, NK_Boolean end_ln, NK_PChar key, NK_PChar fmt, ...);

/**
 * Macro for quickly outputting a "key - 32-bit integer" format row to the terminal table.
 *
 */
#define NK_TermTbl_PutKeyInt32(__Tbl, __end_ln, __key, __int32) \
	do{\
		NK_TermTbl_PutKeyValue(__Tbl, __end_ln, __key, "%d - %08x", (NK_Int)(__int32), (NK_UInt32)(__int32));\
	} while(0)

/**
 * Macro for quickly outputting a "key - string" format row to the terminal table.
 *
 */
#define NK_TermTbl_PutKeyInt64(__Tbl, __end_ln, __key, __int64) \
	do{\
		NK_TermTbl_PutKeyValue(__Tbl, __end_ln, __key, "%lld - %016x", (NK_Int64)(__int64), (NK_UInt64)(__int64));\
	} while(0)

/**
 * Macro for quickly outputting a "key - string" format row to the terminal table.
 *
 */
#define NK_TermTbl_PutKeyString(__Tbl, __end_ln, __key, __str) \
	do{\
		NK_TermTbl_PutKeyValue(__Tbl, __end_ln, __key, "%s", __str);\
	} while(0)


/**
 * End drawing.\n
 * Calling this interface appends a bottom border based on previous output and resets the handle.
 *
 */
NK_API NK_Int
NK_TermTbl_EndDraw(NK_TermTable *Tbl);



NK_CPP_EXTERN_END
#endif /* NK_UTILS_LOG_H_ */
