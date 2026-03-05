ENTRY(_start)

PHDRS
{
    text    PT_LOAD    FLAGS(5); /* R-X */
    rodata  PT_LOAD    FLAGS(4); /* R-- */
    data    PT_LOAD    FLAGS(6); /* RW- */
}

SECTIONS
{
    . = 0xffffffff80000000;

    .text : {
        *(.text .text.*)
    } :text

    /* Force 4K alignment for the segment, 16-byte for SIMD data */
    . = ALIGN(4K);

    .rodata : {
        . = ALIGN(16);
        *(.rodata .rodata.*)
    } :rodata

    .got : {
        *(.got)
        *(.got.plt)
    } :rodata

    . = ALIGN(4K);

    .data : {
        . = ALIGN(16);
        *(.data .data.*)
    } :data

    .bss : {
        . = ALIGN(16);
        *(.bss .bss.*)
        *(COMMON)
    } :data

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}