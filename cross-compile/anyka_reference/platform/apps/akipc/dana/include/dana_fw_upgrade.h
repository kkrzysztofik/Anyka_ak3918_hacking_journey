#ifndef __DANA_FW_UPGRADE_H__
#define __DANA_FW_UPGRADE_H__

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>
#include <stdint.h>

typedef enum _dana_fw_upgrade_err_e_ {
    DANA_FW_UPGRADE_ERR_NULL            =   0x00000000, // succeeded
    DANA_FW_UPGRADE_ERR_FAILED          =   0x00000001, // failed
    DANA_FW_UPGRADE_ERR_UNKNOWN         =   0x00000002, // unknown error
    DANA_FW_UPGRADE_ERR_URL_NOT_SUPPORT =   0x00000003, // URL not supported
    DANA_FW_UPGRADE_ERR_FILE_CORRUPTION =   0x00000005, // file corruption
    DANA_FW_UPGRADE_ERR_NETWORK         =   0x00000006, // network exception
    DANA_FW_UPGRADE_ERR_RW              =   0x00000007, // file write failed
    DANA_FW_UPGRADE_ERR_CANCEL          =   0x00000008, // user cancelled
} dana_fw_upgrade_err_t;



typedef struct _dana_fw_upgrade_callback_funs_s {
    // new firmware callback
    // if return value is 0, the module will start downloading the firmware; need to specify save_pathname as the full path and filename
    uint32_t (*dana_fw_upgrade_new_rom_come)(const uint64_t rom_size, const char *rom_ver, char save_pathname[512]);

    // download complete callback
    void (*dana_fw_upgrade_rom_download_complete)(const uint32_t code, const char *rom_pathname);

    // confirm upgrade callback
    // after successful firmware download, App will ask user to confirm; only after user confirms will the callback be invoked to allow upgrading
    // upgrade_timeout_sec is the estimated upgrade timeout (the longer the better; actual device implementation determines this; generally only a pre-estimated time is needed)
    // need to ensure upgrade process: file replacement + device restart + reboot
    void (*dana_fw_upgrade_confirm)(const char *rom_pathname, uint32_t *upgrade_timeout_sec);
} dana_fw_upgrade_callback_funs_t;



bool dana_fw_upgrade_init(const char *danale_path, const dana_fw_upgrade_callback_funs_t *cbs);

bool dana_fw_upgrade_deinit();

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* __DANA_FW_UPGRADE_H__ */
