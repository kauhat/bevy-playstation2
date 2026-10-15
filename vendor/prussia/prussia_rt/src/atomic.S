# Atomic 64-bit load/store helpers.
#
# Split out of the bootstrap (see start.S) and the CoP0 accessors (see cop0.S).

# The assembler tries to reorder our instructions around delay slots.
# Since we want to preserve this order, disable the reordering feature.
.set noreorder

# Read a u64 atomically. Rust would make two reads, but this would just be the truncated bottom
# half. This method is always correct though.
# $4 (a0) = address.
.global _read_u64
_read_u64:
	nor	$8, $0, $0		# Create a 64-bit all-ones mask in $t0.
	dsrl32	$8, 0			# Turn it into 32-bit.
	ld	$2, 0($4)		# Load from memory into $v0.
	move	$3, $2			# Copy into $v1.
	and	$2, $8			# Return lowest word in $v0.
	dsrl32	$3, 0			# Return highest word in $v1.
	jr	$ra			# And return to caller.
	nop

# Write a u64 atomically. Rust would make two writes, but this would probably confuse the device
# we are writing to. This method is always correct though.
# $4 (a0) = address.
# $5 (a1) = value (lower 32 bits).
# $6 (a2) = value (upper 32 bits).
.global _write_u64
_write_u64:
	dsll32	$6, 0			# upper <<= 32
	or	$5, $6			# lower |= upper
	jr	$ra
	sd	$6, 0($4)		# *address = lower
