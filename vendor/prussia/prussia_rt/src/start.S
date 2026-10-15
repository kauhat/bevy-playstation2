# The assembler tries to reorder our instructions around delay slots.
# Since we want to preserve this order, disable the reordering feature.
.set noreorder

# Bootstrap routine for PruSSia. LLVM only needs a stack pointer, and the rest we can do in Rust.
#
# This lives on its own so that programs which enter through somebody else's crt0 (PS2SDK's
# `__start`, for instance) can still link the CoP0 and atomic helpers from libprussia-rt.a
# without pulling in a second `_start` and the `START_OF_BSS`/`END_OF_BSS` symbols that
# `_rust_start` expects from PruSSia's own linker script.
.extern _rust_start
.global _start
_start:
	j	_rust_start		# Call Rust
	lui	$sp, 0x20		# Set the stack pointer to 0x0020'0000.
