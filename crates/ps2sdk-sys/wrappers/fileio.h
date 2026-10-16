#ifndef WRAPPER_FILEIO_H
#define WRAPPER_FILEIO_H

// `<fileio.h>` refuses to compile against the newlib port ("use posix function calls instead"), but the `fio*`
// routines themselves are still in `libkernel.a` (the link group already carries it). Declare the ones we use.

#include <tamtypes.h>

#ifdef __cplusplus
extern "C" {
#endif

int fioInit(void);
int fioExit(void);
int fioOpen(const char *fname, int mode);
int fioClose(int fd);
int fioRead(int fd, void *buff, int buff_size);
int fioWrite(int fd, const void *buff, int buff_size);
int fioLseek(int fd, int offset, int whence);
int fioSync(const char *devname, int *unused, int *unused2);

#define FIO_O_RDONLY 0x0001
#define FIO_O_WRONLY 0x0002
#define FIO_O_RDWR   0x0003
#define FIO_O_CREAT  0x0200
#define FIO_O_TRUNC  0x0400

#define FIO_SEEK_SET 0
#define FIO_SEEK_CUR 1
#define FIO_SEEK_END 2

#ifdef __cplusplus
}
#endif

#endif
