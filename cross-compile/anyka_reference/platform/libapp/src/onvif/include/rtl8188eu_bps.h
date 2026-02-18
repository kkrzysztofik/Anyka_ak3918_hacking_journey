
#ifndef RTL8188EU_BPS_H_
#define RTL8188EU_BPS_H_

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Get the recommended bitrate (in kbps) for the RTL8188EU solution in STA mode.
 *
 * @return Returns the recommended bitrate on success, or -1 on failure
 *         (possibly because the signal level cannot currently be determined).
 *
 */
extern int RTL8188RecommandBPS();

void APP_WIFI_model_init();

#ifdef __cplusplus
};
#endif

#endif /* RTL8188EU_BPS_H_ */
