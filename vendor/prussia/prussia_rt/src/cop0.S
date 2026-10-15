# Coprocessor 0 register accessors.
#
# These *could* be done with inline assembly, but keeping them here aims for compilation on
# stable.
#
# Split out of the bootstrap (see start.S) so that a program can take just the register helpers.

# The assembler tries to reorder our instructions around delay slots.
# Since we want to preserve this order, disable the reordering feature.
.set noreorder

# Read coprocessor 0 Status register.
.global _read_status
_read_status:
	jr	$ra			# Return to Rust...
	mfc0	$2, $12			# with the Status register value.

# Write coprocessor 0 Status register.
# $4 (a0) = value.
.global _write_status
_write_status:
	jr	$ra			# Return to Rust...
	mtc0	$4, $12			# after setting the Status register.

# Read coprocessor 0 BadVAddr register.
.global _read_badvaddr
_read_badvaddr:
	jr $ra
	mfc0	$2, $8

# Read coprocessor 0 Count register.
.global _read_timercount
_read_timercount:
	jr $ra
	mfc0	$2, $9

# Write coprocessor 0 Count register.
.global _write_timercount
_write_timercount:
	jr	$ra
	mtc0	$4, $9

# Read coprocessor 0 Compare register.
.global _read_compare
_read_compare:
	jr $ra
	mfc0	$2, $11

# Write coprocessor 0 Compare register.
.global _write_compare
_write_compare:
	jr	$ra
	mtc0	$4, $11

# Read coprocessor 0 EPC register.
.global _read_epc
_read_epc:
	jr $ra
	mfc0	$2, $14

# Write coprocessor 0 EPC register.
.global _write_epc
_write_epc:
	jr	$ra
	mtc0	$4, $14

# Read coprocessor 0 BadPAddr register.
.global _read_badpaddr
_read_badpaddr:
	jr $ra
	mfc0	$2, $23

# Write coprocessor 0 BadPAddr register.
.global _write_badpaddr
_write_badpaddr:
	jr	$ra
	mtc0	$4, $23

# Read coprocessor 0 ErrorEPC register.
.global _read_errorepc
_read_errorepc:
	jr $ra
	mfc0	$2, $30

# Write coprocessor 0 ErrorEPC register.
.global _write_errorepc
_write_errorepc:
	jr	$ra
	mtc0	$4, $30

# Read coprocessor 0 Perf/PCCR register.
.global _read_pccr
_read_pccr:
	jr $ra
	mfps	$2, 0

# Write coprocessor 0 Perf/PCCR register.
.global _write_pccr
_write_pccr:
	jr $ra
	mtps	$4, 0

# Read coprocessor 0 PCR0 register.
.global _read_pcr0
_read_pcr0:
	jr $ra
	mfpc	$2, 0

# Write coprocessor 0 PCR0 register.
.global _write_pcr0
_write_pcr0:
	jr $ra
	mtpc	$4, 0

# Read coprocessor 1 PCR1 register.
.global _read_pcr1
_read_pcr1:
	jr $ra
	mfpc	$2, 1

# Write coprocessor 1 PCR1 register.
.global _write_pcr1
_write_pcr1:
	jr $ra
	mtpc	$4, 1
