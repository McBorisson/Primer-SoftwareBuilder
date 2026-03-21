# Build Your Own Operating System

This is the v0.1 canonical recipe used to validate contract, adapter, and milestone flow.

## Prerequisites

- `nasm`
- `gcc`
- `qemu-system-i386`
- `make`

## Milestones

1. Bootloader
2. Kernel entry
3. VGA output
4. Interrupts
5. Memory
6. Scheduler
7. Filesystem
8. Shell

## Validation

Run:

```bash
scripts/validate-recipe recipes/operating-system
```
