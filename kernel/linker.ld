/* Tell the linker the entry point is _start */
ENTRY(_start)

PHDRS
{
    text    PT_LOAD    FLAGS(5); /* Read + Execute */
    rodata  PT_LOAD    FLAGS(4); /* Read Only */
    data    PT_LOAD    FLAGS(6); /* Read + Write */
}

SECTIONS
{
    /* Map the kernel to the high-half (Top 2GB) */
    . = 0xffffffff80000000;

    .text : {
        *(.text .text.*)
    } :text

    . = ALIGN(4K);

    .rodata : {
        *(.rodata .rodata.*)
    } :rodata

    . = ALIGN(4K);

    .data : {
        *(.data .data.*)
    } :data

    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    } :data
}