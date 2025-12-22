# Nexus Keylogger BOF Build Configuration

CC=cl.exe
CFLAGS=/c /GS- /Gs9999999 /O2 /MT /kernel
LDFLAGS=/DLL /NODEFAULTLIB /SUBSYSTEM:WINDOWS /MACHINE:X64

TARGET=nexus_keylogger.o
SOURCE=nexus_keylogger.c

all: $(TARGET)

$(TARGET): $(SOURCE)
	$(CC) $(CFLAGS) $(SOURCE) /Fo:$(TARGET)

clean:
	del /f /q *.o *.obj *.exp *.lib 2>nul

.PHONY: all clean
