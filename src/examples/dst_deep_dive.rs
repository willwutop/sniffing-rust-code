fn bad_dst_code() {
    // ❌ the size for values of type `str` cannot be known at compilation time
    // ❌ type mismatch resolving `<i32 as FromStr>::Err == dyn Error`
    // ❌ the size for values of type `dyn std::error::Error` cannot be known at compilation time
    // fn dst_function(s: str) -> Result<i32, dyn std::error::Error> {
    //     s.parse()
    // }
}

#[test]
// cargo test --lib -F dst-deep-dive -- test_fat_pointer_size --nocapture
fn test_fat_pointer_size() {
    // 数组切片 - 胖指针
    let arr = [1, 2, 3, 4, 5];
    let slice: &[i32] = &arr[1..4];
    // 通过unsafe代码查看切片的内部结构
    #[repr(C)]
    struct SliceRepr<T> {
        data: *const T,
        len: usize,
    }
    let repr = unsafe { &*(&raw const slice as *const SliceRepr<i32>) };

    println!("&str size: {}", std::mem::size_of::<&str>()); // 16字节 👈 比普通指针大一倍
    println!("&[i32] size: {}", std::mem::size_of::<&[i32]>()); // 16字节 👈 包含长度信息
    println!("&i32 size: {}", std::mem::size_of::<&i32>()); // 8字节
    println!("*const i32 size: {}", std::mem::size_of::<*const i32>()); // 8字节

    println!("slice ptr address: {:p}", slice.as_ptr()); // slice ptr address: 0x7944247fe2f8
    println!("slice data address: {:p}, len: {}", repr.data, repr.len); // slice data address: 0x7944247fe2f8, len: 3
}

mod smart_ptr {
    use std::cell::{Cell, RefCell};
    use std::error::Error;
    use std::ops::Deref;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::{fmt, thread};

    // 自定义错误类型
    #[derive(Debug)]
    struct NetworkError(String);

    impl fmt::Display for NetworkError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "网络错误: {}", self.0)
        }
    }

    impl Error for NetworkError {}

    #[derive(Debug)]
    struct ParseError(String);

    impl fmt::Display for ParseError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "解析错误: {}", self.0)
        }
    }

    impl Error for ParseError {}

    // ✅ 使用Box<dyn Error>统一处理不同错误类型
    fn process_data(input: &str) -> Result<i32, Box<dyn Error>> {
        if input.is_empty() {
            return Err(Box::new(NetworkError("输入为空".to_string())));
        }

        match input.parse::<i32>() {
            Ok(num) => Ok(num),
            Err(_) => Err(Box::new(ParseError(format!("无法解析: {}", input)))),
        }
    }

    #[test]
    // cargo test --lib -F dst-deep-dive -- test_process_data --nocapture
    fn test_process_data() {
        assert_eq!(process_data("123").unwrap(), 123);
        assert!(process_data("no a num").is_err());
        println!("Box<dyn Error> size: {}", size_of::<Box<dyn Error>>()) // 16字节 👈 [data_ptr, vtable_ptr]
    }

    fn smart_pointers_in_practice() {
        // Rc 配置共享场景：多个组件需要同一份配置
        let config = Rc::new(String::from("production"));
        // let db_manager = DatabaseManager::new(Rc::clone(&config));
        // let api_server = ApiServer::new(Rc::clone(&config));

        println!("配置被引用了 {} 次", Rc::strong_count(&config)); // 3次

        // Arc 多线程场景：需要线程安全的共享
        let shared_counter = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let counter = Arc::clone(&shared_counter);
                thread::spawn(move || {
                    // 模拟一些工作
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    println!("线程 {} 完成工作", i);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // RefCell<T> - 内部可变性
        let mutable_in_immutable = RefCell::new(vec![1, 2, 3]);
        mutable_in_immutable.borrow_mut().push(4);
    }

    // 简单的调试智能指针 - 记录访问次数
    struct DebugBox<T> {
        data: Box<T>,
        access_count: Cell<usize>, // 👈 内部可变性，允许在不可变引用中修改
    }

    impl<T> DebugBox<T> {
        fn new(data: T) -> Self {
            Self {
                data: Box::new(data),
                access_count: Cell::new(0),
            }
        }

        fn access_count(&self) -> usize {
            self.access_count.get()
        }
    }

    // ✨ 实现Deref让智能指针表现得像普通引用
    impl<T> Deref for DebugBox<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.access_count.update(|c| c + 1);
            &self.data
        }
    }

    impl<T: fmt::Display> fmt::Display for DebugBox<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let data: &T = &self;
            write!(f, "DebugBox({})", data)
        }
    }

    impl<T> Drop for DebugBox<T> {
        fn drop(&mut self) {
            println!("🗑️  销毁DebugBox，总共访问了{}次", self.access_count.get());
        }
    }

    #[test]
    // cargo test --lib -F dst-deep-dive -- test_debug_box --nocapture
    fn test_debug_box() {
        let debug_str = DebugBox::new("Hello, Rust!".to_string());

        println!("字符串长度: {}", debug_str.len()); // 触发deref
        println!("内容: {}", *debug_str); // 再次触发deref
        println!("显示: {}", debug_str); // Display实现会触发1次deref

        println!("总访问次数: {}", debug_str.access_count()); // 3
        // 离开作用域时自动调用drop
    }
}

mod dyn_processor {
    use std::error::Error;

    // 定义插件接口
    trait DataProcessor {
        fn name(&self) -> &str;
        fn process(&self, data: &str) -> Result<String, Box<dyn Error>>;
        fn priority(&self) -> u8 {
            5
        } // 默认优先级
    }

    // JSON处理器
    struct JsonProcessor;
    impl DataProcessor for JsonProcessor {
        fn name(&self) -> &str {
            "JSON处理器"
        }

        fn process(&self, data: &str) -> Result<String, Box<dyn Error>> {
            // 简化的JSON处理逻辑
            if data.starts_with('{') && data.ends_with('}') {
                Ok(format!("已处理JSON数据: {}", data))
            } else {
                Err("不是有效的JSON格式".into())
            }
        }

        fn priority(&self) -> u8 {
            8
        }
    }

    // CSV处理器
    struct CsvProcessor;
    impl DataProcessor for CsvProcessor {
        fn name(&self) -> &str {
            "CSV处理器"
        }

        fn process(&self, data: &str) -> Result<String, Box<dyn Error>> {
            let lines: Vec<&str> = data.lines().collect();
            if lines.len() > 1 {
                Ok(format!("已处理{}行CSV数据", lines.len()))
            } else {
                Err("CSV数据格式不正确".into())
            }
        }

        fn priority(&self) -> u8 {
            6
        }
    }

    // ✨ 插件管理器：这里是DST的核心应用
    struct PluginManager {
        processors: Vec<Box<dyn DataProcessor>>, // 👈 不同类型的处理器统一管理
    }

    impl PluginManager {
        fn new() -> Self {
            Self {
                processors: Vec::new(),
            }
        }

        fn register<T>(&mut self, processor: T)
        where
            T: DataProcessor + 'static, // 👈 'static保证生命周期安全
        {
            self.processors.push(Box::new(processor));
            // 按优先级排序
            self.processors
                .sort_by(|a, b| b.priority().cmp(&a.priority()));
        }

        fn process_data(&self, data: &str) -> Result<String, Box<dyn Error>> {
            for processor in &self.processors {
                println!(
                    "尝试使用 {} (优先级: {})",
                    processor.name(),
                    processor.priority()
                );
                match processor.process(data) {
                    Ok(result) => {
                        println!("✅ {} 处理成功", processor.name());
                        return Ok(result);
                    }
                    Err(e) => {
                        println!("❌ {} 处理失败: {}", processor.name(), e);
                        continue;
                    }
                }
            }
            Err("所有处理器都无法处理该数据".into())
        }
    }

    #[test]
    // cargo test --lib -F dst-deep-dive -- test_plugin_system --nocapture
    fn test_plugin_system() {
        let mut manager = PluginManager::new();

        // ✨ 注册不同类型的处理器
        manager.register(JsonProcessor);
        manager.register(CsvProcessor);

        // 测试JSON数据
        let json_data = r#"{"name": "张三", "age": 30}"#;
        match manager.process_data(json_data) {
            Ok(result) => println!("处理结果: {}", result),
            Err(e) => println!("处理失败: {}", e),
        }
        // 测试CSV数据
        let csv_data = "姓名,年龄\n张三,30\n李四,25";
        match manager.process_data(csv_data) {
            Ok(result) => println!("处理结果: {}", result),
            Err(e) => println!("处理失败: {}", e),
        }
    }
}

mod dst_examples {
    fn dst_init() {
        // ❌ 错误：不能直接创建DST
        // let s: str = "hello";

        // ✅ 正确：通过引用或智能指针
        let s: &str = "hello"; // 只读访问，无数据所有权
        let s: Box<str> = "hello".into(); // 只读访问，拥有数据所有权
        let s: String = "hello".to_string(); // 读写访问，拥有数据所有权
    }

    fn dst_lifetime() {
        // ❌ 返回局部变量的引用
        // fn lifetime_trouble() -> &str {
        //     let local_string = String::from("hello");
        //     &local_string  // 💥 每个Rust新手都会犯的错
        // }

        // ✅ 几种正确的解决方案
        fn lifetime_solutions() -> String {
            // 方案1: 返回拥有所有权的类型
            String::from("hello")
        }

        fn lifetime_solutions2() -> &'static str {
            // 方案2: 返回静态字符串
            "hello"
        }

        fn lifetime_solutions3() -> Box<str> {
            // 方案3: 使用智能指针
            "hello".into()
        }
    }

    // ❌ 为什么这些不能用于trait对象？
    trait ProblematicTrait {
        // 1. 返回Self - 调用者不知道具体类型大小
        fn clone_self(&self) -> Self;

        // 2. 泛型方法 - 无法在vtable中确定泛型实例
        fn generic_method<T>(&self, param: T);

        // 3. 关联类型 - 同一个trait可能有多个实现，关联类型不确定
        type Output;
        fn get_output(&self) -> Self::Output;

        // 4. 静态方法 - 没有self参数，无法通过vtable调用
        fn static_method();
    }

    // ✅ 对象安全的trait设计
    trait ObjectSafe {
        fn method_with_self(&self);
        fn method_with_known_return(&self) -> i32;
        fn method_with_box_return(&self) -> Box<dyn std::fmt::Display>;
    }

    fn collection_choice_guide() {
        // ✅ 编译时已知大小 -> 数组
        let coordinates: [f64; 3] = [1.0, 2.0, 3.0];

        // ✅ 运行时动态增长 -> Vec
        let mut dynamic_list = Vec::new();
        dynamic_list.push("item1");
        dynamic_list.push("item2");

        // ✅ 只读访问现有数据 -> 切片
        fn process_items(items: &[&str]) {
            // 👈 接受任意长度的切片
            for item in items {
                println!("处理: {}", item);
            }
        }

        let coord_strings: Vec<String> = coordinates.iter().map(|x| x.to_string()).collect();
        let coord_refs: Vec<&str> = coord_strings.iter().map(|s| s.as_str()).collect();
        process_items(&coord_refs); // 数组转切片
        process_items(&dynamic_list); // Vec转切片
    }
}

mod error_handling_patterns {
    use std::error::Error;

    // ✅ 简单场景：使用具体错误类型
    fn simple_parse(input: &str) -> Result<i32, std::num::ParseIntError> {
        input.parse()
    }

    // ✅ 多种错误类型：使用trait对象
    fn complex_operation(input: &str) -> Result<Data, Box<dyn Error>> {
        let num = input.parse::<i32>()?; // ParseIntError
        let data = fetch_data(num)?; // NetworkError
        validate_data(&data)?; // ValidationError
        Ok(data)
    }

    // ✅ 性能关键场景：使用枚举
    #[derive(Debug)]
    enum AppError {
        Parse(std::num::ParseIntError),
        Network(String),
        Validation(String),
    }

    fn performance_critical(input: &str) -> Result<Data, AppError> {
        let num = input.parse().map_err(AppError::Parse)?;
        // ... 其他处理
        Ok(Data { value: num })
    }

    struct Data {
        value: i32,
    }
    fn fetch_data(_: i32) -> Result<Data, Box<dyn Error>> {
        Ok(Data { value: 42 })
    }
    fn validate_data(_: &Data) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

mod dst_usage {
    use std::fmt::Display;

    fn str_distinction_demo() {
        // &str - 对str的借用，16字节胖指针
        let borrowed: &str = "hello"; // 借用字符串字面量
        let slice: &str = &String::from("world")[..]; // 借用String的一部分

        // String - 拥有所有权的字符串，包含堆分配的缓冲区
        let owned: String = "hello".to_string(); // 拥有数据

        // Box<str> - 拥有所有权但不可增长的字符串
        let boxed: Box<str> = "hello".into(); // 拥有数据，但固定大小
    }

    fn dst_simple_usage() {
        // 从简单到复杂，按需使用
        let simple: [i32; 3] = [1, 2, 3]; // 零开销
        let dynamic: Vec<i32> = vec![1, 2, 3]; // 明确的堆分配开销
        let polymorphic: Vec<Box<dyn Display>> = vec![
            // 最大灵活性，明确的vtable开销
            Box::new(42),
            Box::new("hello"),
        ];
    }

    // safe代码：编译器保证绝对安全
    fn safe_operations(data: &[i32]) {
        let item = data.get(10); // 返回Option，不会panic
        if let Some(value) = item {
            println!("值: {}", value);
        }
    }

    // unsafe代码：开发者承担安全责任，但边界明确
    fn unsafe_operations(data: &[i32]) {
        unsafe {
            let ptr = data.as_ptr();
            let value = *ptr.add(10); // 🚨 你需要保证这里不会越界
            println!("值: {}", value);
        }
    }
}
