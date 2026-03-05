The upgrade tool is mainly used to upgrade the kernel and other files placed in the bin area (such as logo), boot, and ram parameters. It supports local upgrade, http upgrade, and ftp upgrade.

1 Local Upgrade
Copy the upgrade file to the file system or SD card, and then execute the upgrade command:
updater local K=<kernel path> B=<boot path> L=<logo path> D=<ram parameter path>
Example: updater local K=/mnt/sd/zImage B=/mnt/sd/nandboot.bin D=/mnt/sd/ddrpar.txt

2 HTTP Upgrade
Put the upgrade file on the HTTP server, ensure the development board can connect to the server, and then execute the command:
updater http K=<kernel path> B=<boot path> L=<logo path> D=<ram parameter path> X=<0/1>
Example: updater http K=http://www.a.com/zImage B=http://www.a.com/nandboot.bin D=http://www.a.com/ddrpar.txt X=1

3 FTP Upgrade
Put the upgrade file on the FTP server, ensure the development board can connect to the server, and then execute the command:
updater ftp K=<kernel path> B=<boot path> L=<logo path> D=<ram parameter path> X=<0/1> A=<ip addr> P=<port> U=<username> C=<password>
Example: updater ftp K=/update/zImage B=/update/nandboot.bin D=/update/ddrpar.txt X=0 A=192.168.1.100 P=21 U=anonymous C=anonymous
Where A is the FTP server IP address, P is the FTP port number, U is the username, and C is the password.

After the command is executed, a series of warning messages will pop up. Enter OK and then press Enter to continue, otherwise the execution will be terminated.
During the upgrade process, progress prompts and information on success or failure of each stage ("update ..... success/failure") will pop up. Finally, "Update End! You Should Reboot The System" will pop up. At this time, you need to restart your development board. As long as it can start normally, the upgrade is generally successful.

Notes:
1 K, B, and L options can be selected all or just one of them, but the D option must depend on the B option, which means that when upgrading ram parameters, you must upgrade boot at the same time.
2 The format of the ram parameter file is consistent with the format exported by the burning tool.
3 In http upgrade and ftp upgrade, X is whether to verify the option. The value is 1 (default) to choose verification. If verification is required, the upgrade file needs to be processed with software on the PC side.
