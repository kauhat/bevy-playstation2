// Core PS2 types
#include <tamtypes.h>

// EE Kernel (semaphores, threads, etc.)
#include <kernel.h>

// Debug output
#include <debug.h>

// TODO: is this a hack?
extern int printf(const char *format, ...);
extern int getchar(void);
extern int putchar(int c);
extern int puts(const char *s);
extern char *gets(char *s);
extern int fdprintf(int fd, const char *format, ...);
extern int fdgetc(int fd);
extern int fdputc(int c, int fd);
extern int fdputs(const char *s, int fd);
extern char *fdgets(char *buf, int fd);
