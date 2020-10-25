use ggbc::ir::Ir;
use ggbc_vm::VM;
use ggbc::parser::Ast;

static INPUT: &str = indoc::indoc! {r#"
    // small program that counts up to 42
    static@0x0000 RESULT:u8

    loop {
        let should_add_one:u8 = (- 42 RESULT)

        if should_add_one {
            let tmp:u8 = (+ 1 RESULT)
            (= RESULT tmp)
        } else {
            break
        }
    }
"#};

fn print_input(input: &str, ast: &Ast) {
    println!("Input");
    println!("===");
    println!("{}", input);
    #[cfg(nope)]
    {
        println!();
        println!("Ast");
        println!("===");
        println!("{:?}", ast);
    }
}

fn print_ir(ir: &Ir) {
    println!();
    println!("Ir");
    println!("===");
    for (i, statement) in ir.routines[ir.main].statements.iter().enumerate() {
        println!("{:04x} | {:?}", i, statement);
    }
}

fn run_vm(mut vm: VM) {
    println!();
    println!("Cycles");
    println!("===");
    let mut cycles = 0;
    while vm.running() {
        #[cfg(nope)]
        println!("pc = {:x}", vm.pc());
        vm.update();
        cycles += 1;

        #[cfg(nope)]
        if cycles == 64 {
            println!("break");
            break;
        }
    }
    println!("\nRan for {} cycles", cycles);

    println!();
    println!("Result");
    println!("===");
    const OUTPUT: usize = 8;
    for (addr, b) in vm.absolute()[..OUTPUT].iter().enumerate() {
        println!("{:04x} | {:02x} ({})", addr, b, b);
    }
}

fn main() {
    // run program
    let ast = ggbc::parser::parse(INPUT).unwrap();
    // display program
    print_input(INPUT, &ast);

    let ir = ggbc::ir::compile(&ast);
    print_ir(&ir);

    run_vm(VM::new(ir));
}