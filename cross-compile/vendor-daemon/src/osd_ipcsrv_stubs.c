/*
 * No-op stubs for symbols that libmpi_osd.so resolves via osd_sys_ipc_register.
 *
 * Our shipped libplat_ipcsrv.so exports ak_cmd_server_register / 
 * ak_cmd_register_msg_handle, but not ak_cmd_register_module.  ak_osd_init
 * always calls osd_sys_ipc_register at the end (verified by disassembly of
 * libmpi_osd V1.1.03 at ak_osd_init+0x230), which then calls these two.
 *
 * The vendor remote-command table is unused by our daemon IPC, so returning
 * without registering is correct.  Without these stubs the process dies at
 * the PLT the first time ak_osd_init runs.
 */

void ak_cmd_register_module(unsigned int port, const char *name)
{
    (void)port;
    (void)name;
}

void ak_cmd_unregister_module(unsigned int port, const char *name)
{
    (void)port;
    (void)name;
}
