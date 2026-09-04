.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    csrrw sp, sscratch, sp
    # Now sp -> kernel stack, sscratch -> user stack.
    # Allocate a TrapContext on kernel stack.
    addi sp, sp, -34*8
    # Save general-purpose registers.
    sd x1, 1*8(sp)
    # Skip sp(x2), we will save it later.
    sd x3, 3*8(sp)
    # Skip tp(x4), application does not use it.
    # Save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # We can use t0/t1/t2 freely, because they were saved on kernel stack.
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # Read user stack from sscratch and save it on the kernel stack.
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # Set input argument of trap_handler(cx: &mut TrapContext)
    mv a0, sp
    call trap_handler

__restore:
    # Case 1: start running app by __restore.
    # Case 2: back to U after handling trap.
    mv sp, a0
    # Now sp -> kernel stack (after allocated), sscratch -> user stack.
    # Restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    # Restore general-purpose registers except sp/tp.
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # Release TrapContext on kernel stack.
    addi sp, sp, 34*8
    # Now sp -> kernel stack, sscratch -> user stack.
    csrrw sp, sscratch, sp
    sret
