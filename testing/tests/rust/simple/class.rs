use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn DataCorruption_Handler() {
    println!("ASPIS_FAULT_INJECTION_CAUGHT: DataCorruption_Handler");
    std::io::stdout().flush().unwrap();
}

#[unsafe(no_mangle)]
pub extern "C" fn SigMismatch_Handler() {
    println!("ASPIS_FAULT_INJECTION_CAUGHT: SigMismatch_Handler");
    std::io::stdout().flush().unwrap();
}

struct MyClass {
    a: i32,
    b: i32,
}

impl MyClass {
    fn sum(&self) -> i32 {
        self.a + self.b
    }

    fn print(&self) {
        println!("{}, {}", self.a, self.b);
    }
}

struct DerivedClass {
    base: MyClass,
    c: i32,
}

impl DerivedClass {
    fn print(&self) {
        println!("{}, {}, {}", self.base.a, self.base.b, self.c);
    }
}

#[unsafe(link_section = "aspis_to_harden")]
static mut DERIVED_OBJ: DerivedClass = DerivedClass {
    base: MyClass { a: 3, b: 6 },
    c: 9,
};

fn main() {
    // Test class and member function
    let my_obj = MyClass { a: 5, b: 7 };
    println!("{}", my_obj.sum());
    my_obj.print();

    // Test derived class with overridden "virtual" function
    let derived_obj = unsafe { &*(&raw const DERIVED_OBJ) };
    derived_obj.print();
}
