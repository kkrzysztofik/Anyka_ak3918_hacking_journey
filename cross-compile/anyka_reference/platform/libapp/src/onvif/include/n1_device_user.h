
/**
 * @brief
 *  N1 user management event and interface definitions.
 * @details
 *  Users implement user management event responses through this module to
 *  achieve the corresponding functionality.\n
 *  The module loads events via the @ref NK_N1Device_UserManage() interface.\n
 *  This interface must be called only after @ref NK_N1Device_Init() has been
 *  called successfully; otherwise it will fail.
 *
 */

#include "n1_device.h"

#ifndef NK_N1_DEVICE_USER_H_
#define NK_N1_DEVICE_USER_H_
NK_CPP_EXTERN_BEGIN


/**
 * @macro
 *  Maximum user name length definition.
 */
#define NK_N1_USER_MAX_NAME_SZ    (64)

/**
 * @macro
 *  Maximum user password length definition.
 */
#define NK_N1_USER_MAX_PASS_SZ    (NK_N1_USER_MAX_NAME_SZ)


/**
 * @brief
 *  User privilege level definitions.
 */
typedef enum Nk_N1UserClassify {

	NK_N1_USR_CLASS_ADMIN = (0),//!< NK_N1_USR_CLASS_ADMIN
	NK_N1_USR_CLASS_VIEWER,     //!< NK_N1_USR_CLASS_VIEWER

} NK_N1UserClassify;


/**
 * @brief
 *  Related response event definitions.
 */
typedef struct Nk_N1DeviceEventUserManage {

	NK_Void
	(*onAdd)(NK_PVoid ctx, const NK_PChar username, const NK_PChar passphrase, NK_N1UserClassify classify);

	NK_Void
	(*onRemove)(NK_PVoid ctx, const NK_PChar username);

	NK_Void
	(*onEdit)(NK_PVoid ctx, const NK_PChar username, const NK_PChar passphrase, NK_N1UserClassify classify);

} NK_N1DeviceEventUserManage;

/**
 * @brief
 *  Configure related extension events.
 * @details
 *
 * @param Event [in]
 *  User event definition.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_UserManage(NK_N1DeviceEventUserManage *Event);


/**
 * @brief
 *  Add an authenticated user.\n
 *
 * @param username [in]
 *  The user's username.
 * @param passphrase [in]
 *  The user's authentication password.
 * @param classify [in]
 *  User privilege level.
 *
 * @return
 *  Returns 0 on success, -1 on failure.
 */
NK_API NK_Int
NK_N1Device_AddUser(const NK_PChar username, const NK_PChar passphrase, NK_N1UserClassify classify);

/**
 * @macro
 *  Edit/update user.
 */
#define NK_N1Device_EditUser NK_N1Device_AddUser


/**
 * Delete an authenticated user.
 *
 * @param[in]			username			The user's username.
 *
 * @return		Returns 0 on success, -1 on failure.
 */
NK_API NK_Void
NK_N1Device_RemoveUser(const NK_PChar username);

/**
 * Get information for the specified username.\n
 * If the user exists, returns the user's password @ref passphrase and
 * forbidden permission flag @ref forbiden_r.
 *
 * @param[in]			username			The user's username.
 * @param[out]			passphrase			The user's authentication password.
 * @param[out]			forbidden_r			The user's forbidden permission flag.
 *
 * @return		Returns True if the user exists (with info in @ref passphrase and
 *              @ref forbidden_r), or False if the user does not exist.
 */
NK_API NK_Boolean
NK_N1Device_HasUser(const NK_PChar username, NK_PChar passphrase, NK_N1UserClassify *classify);

/**
 * @brief
 *  Look up a user by index.
 *
 * @param id [in]
 *  User index, starting from 0 in the order users were added.
 * @param username
 * @param passphrase
 * @param classify
 * @return
 */
NK_API NK_Boolean
NK_N1Device_IndexUser(NK_Int id, NK_PChar username, NK_PChar passphrase, NK_N1UserClassify *classify);


/**
 * Get the number of authenticated users.
 *
 * @return		Number of users. If the count is 0, no authentication is performed
 *              for network communication.
 */
NK_API NK_Size
NK_N1Device_NumberOfUser();



NK_CPP_EXTERN_END
#endif /* NK_N1_DEVICE_USER_H_ */
