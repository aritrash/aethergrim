KERNEL := kernel/target/x86_64-aether/debug/kernel
ISO := aether-grim.iso
TOOLCHAIN := nightly-x86_64-pc-windows-msvc

# Force the compiler to use soft-float at the highest priority
export RUSTFLAGS := -C target-feature=-mmx,-sse,-sse2,-sse3,-ssse3,-sse4.1,-sse4.2,-3dnow,-3dnowa,-avx,-avx2,+soft-float

.PHONY: all run clean

$(KERNEL):
	cd kernel && RUSTFLAGS="" cargo +nightly build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z json-target-spec \
		--target x86_64-aether.json

$(ISO): $(KERNEL)
	mkdir -p iso_root
	cp target/x86_64-aether/debug/kernel iso_root/kernel.elf
	cp limine.cfg iso_root/
	cp limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/
	xorriso -as mkisofs -b limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(ISO)

run: $(ISO)
	qemu-system-x86_64 -cdrom aether-grim.iso -vga std -serial stdio -m 512M