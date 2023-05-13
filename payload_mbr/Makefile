all: build run
clean:
	rm -rf *.bin *.img
build:
	nasm -o disk.img kernel.asm
run:
	qemu-system-i386 -fda disk.img
