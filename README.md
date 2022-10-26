# msp432p401r-gate-tester

This is the firmware for an 4-gate AND/OR chip tester for the TI MSP432P401R microcontroller, written in rust. It makes use of the 'msp432p401r-hal' crate.

The interrupts allow you to use the P1_1 and P1_4 buttons to change the gate type and which gate on a 4-gate chip you are testing during operation.
