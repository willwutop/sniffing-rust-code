mod walk_iter {
    // ❌ 错误：使用索引操作
    fn use_index_walk_iter(vec: &[i32]) {
        for i in 0..vec.len() {
            println!("{}", vec[i]);
        }
    }

    // ✅ 使用迭代器: for
    fn use_iter_for_walk(vec: &[i32]) {
        for item in vec.iter() {
            println!("{}", item);
        }
        // 对于需要索引的场景使用 enumerate
        for (idx, item) in vec.iter().enumerate() {
            println!("{}-{}", idx, item);
        }
    }

    // ✅ 使用迭代器: while
    // 更适合需要手动控制的场景
    fn use_iter_while_walk(vec: &[&str]) {
        let mut iter = vec.iter();
        while let Some(&key) = iter.next()
            && let Some(&value) = iter.next()
        {
            println!("{key}: {value}",);
        }
    }

    #[test]
    // cargo test --lib -F iterator-patterns -- test_use_iter_while_walk --nocapture
    fn test_use_iter_while_walk() {
        use_iter_while_walk(&["key1", "value1", "key2", "value2", "key3"]);
    }
}

mod impl_my_iter_ext {
    // 不可变引用迭代器
    pub struct Iter<'a, T> {
        slice: &'a [T],
        idx: usize,
    }

    impl<'a, T> Iterator for Iter<'a, T> {
        type Item = &'a T;

        fn next(&mut self) -> Option<Self::Item> {
            if self.idx == self.slice.len() {
                None
            } else {
                let idx = self.idx;
                self.idx += 1;
                self.slice.get(idx)
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let len = self.slice.len() - self.idx;
            (len, Some(len))
        }
    }

    impl<'a, T> From<&'a [T]> for Iter<'a, T> {
        fn from(slice: &'a [T]) -> Self {
            Self { slice, idx: 0 }
        }
    }

    // 迭代器功能拓展
    pub trait MyIter: Iterator {
        fn my_map<F, R>(self, f: F) -> Map<Self, F>
        where
            Self: Sized,
            F: Fn(Self::Item) -> R,
        {
            Map { iter: self, f }
        }

        fn my_filter<F>(self, f: F) -> Filter<Self, F>
        where
            Self: Sized,
            F: Fn(&Self::Item) -> bool,
        {
            Filter { iter: self, f }
        }
    }

    // 自定义迭代器器适配
    impl<T> MyIter for Iter<'_, T> {}

    impl<I, F> MyIter for Map<I, F> where Self: Iterator {}

    impl<I, F> MyIter for Filter<I, F> where Self: Iterator {}

    // map 迭代器
    pub struct Map<I, F> {
        iter: I,
        f: F,
    }

    impl<I, F, T> Iterator for Map<I, F>
    where
        I: Iterator,
        F: Fn(I::Item) -> T,
    {
        type Item = T;
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(item) = self.iter.next() {
                Some((self.f)(item))
            } else {
                None
            }
        }
    }

    // filter 迭代器
    pub struct Filter<I, F> {
        iter: I,
        f: F,
    }

    impl<I, F> Iterator for Filter<I, F>
    where
        I: Iterator,
        F: Fn(&I::Item) -> bool,
    {
        type Item = I::Item;

        fn next(&mut self) -> Option<Self::Item> {
            for item in &mut self.iter {
                if (self.f)(&item) {
                    return Some(item);
                }
            }
            None
        }
    }

    #[test]
    // cargo test --lib -F iterator-patterns -- test_iter_map_filter --nocapture
    fn test_iter_map_filter() {
        let vec = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let iter = Iter::from(vec.as_slice())
            .my_map(|i| {
                println!("my-map: {}", i);
                i * i
            })
            .my_filter(|i| i % 2 == 0);
        println!("start iter ... ");
        let res: Vec<_> = iter.collect();
        println!("res: {:?}", res);
    }
}

#[test]
// cargo test --lib -F iterator-patterns -- zero_cost_benchmark --nocapture
fn zero_cost_benchmark() {
    use std::time::Instant;
    const SIZE: usize = 1_000_000;
    let data: Vec<i32> = (0..SIZE as i32).collect();

    // 手写循环版本
    let start = Instant::now();
    let mut sum1 = 0i64;
    for i in 0..data.len() {
        if data[i] % 2 == 0 {
            sum1 += (data[i] * 2) as i64;
        }
    }
    let manual_time = start.elapsed();

    // 迭代器版本
    let start = Instant::now();
    let sum2: i64 = data
        .iter()
        .filter(|&&x| x % 2 == 0)
        .map(|&x| (x * 2) as i64)
        .sum();
    let iterator_time = start.elapsed();

    println!("手写循环结果: {}, 耗时: {:?}", sum1, manual_time);
    println!("迭代器结果: {}, 耗时: {:?}", sum2, iterator_time);
    println!(
        "性能差异: {:.2}%",
        (iterator_time.as_nanos() as f64 / manual_time.as_nanos() as f64 - 1.0) * 100.0
    );

    assert_eq!(sum1, sum2);
}

mod iter_best_practice {
    use std::error::Error;
    use std::num::ParseIntError;

    // 最佳实践1：合理使用size_hint进行预分配
    fn best_practice_preallocation() {
        let data: Vec<i32> = (0..1000).collect();

        // ❌ 让Vec自动扩容
        let result1: Vec<i32> = data
            .iter()
            .filter(|&&x| x % 2 == 0)
            .map(|&x| x * 2)
            .collect();

        // ✅ 利用size_hint预分配容量
        let filtered_iter = data.iter().filter(|&&x| x % 2 == 0);
        let (min_size, _) = filtered_iter.size_hint();
        let mut result2 = Vec::with_capacity(min_size);

        for &x in data.iter().filter(|&&x| x % 2 == 0) {
            result2.push(x * 2);
        }

        println!("结果1长度: {}, 结果2长度: {}", result1.len(), result2.len());
    }

    // 最佳实践2：使用Result/Option作为结果类型
    fn best_practice_result_option() -> Result<Vec<i32>, ParseIntError> {
        let data = vec!["invalid", "1", "2", "3", "invalid", "5"];
        data.into_iter()
            .map(str::parse) // 👈 映射为Result<i32, ParseIntError>
            .collect::<Result<Vec<_>, _>>() // 👈 遇到Err就会中断执行并返回错误
    }

    // 最佳实践3：使用 try_fold 进一步优化错误处理
    fn best_practice_error_handling() -> Result<i32, Box<dyn Error>> {
        let data = vec!["1", "2", "3", "invalid", "5"];

        // ❌ 使用collect完成后继续计算，重复collect
        let parsed: Result<Vec<i32>, _> = data.iter().map(|s| s.parse::<i32>()).collect();
        let _res: Result<i32, Box<dyn Error>> = Ok(parsed?.iter().sum::<i32>());

        // ✅ 使用try_fold，遇到错误立即停止
        data.iter().try_fold(0, |acc, s| {
            s.parse::<i32>().map(|n| acc + n).map_err(From::from)
        })
    }

    // 最佳实践4：组合多个数据源
    fn best_practice_multiple_sources() {
        let users = vec!["Alice", "Bob", "Charlie"];
        let scores = vec![95, 87, 92];
        let active = vec![true, false, true];

        // 使用zip组合多个迭代器
        let active_users: Vec<(&str, i32)> = users
            .iter()
            .zip(scores.iter())
            .zip(active.iter())
            .filter(|(_, is_active)| **is_active)
            .map(|((&name, &score), _)| (name, score))
            .collect();

        println!("活跃用户: {:?}", active_users);
    }

    // 最佳实践5：自定义容器类型（实现FromIterator）进行collect
    fn best_practice_custom_collect() {
        let data = vec!["apple", "banana", "cherry", "date"];

        // 收集到不同的容器类型
        use std::collections::{BTreeSet, HashSet};

        let vec_result: Vec<&str> = data.iter().cloned().collect();
        let set_result: HashSet<&str> = data.iter().cloned().collect();
        let btree_result: BTreeSet<&str> = data.iter().cloned().collect();

        println!("Vec: {:?}", vec_result);
        println!("HashSet: {:?}", set_result);
        println!("BTreeSet: {:?}", btree_result);

        // 收集到字符串
        let joined: String = data
            .iter()
            .enumerate()
            .map(|(i, &fruit)| format!("{}. {}", i + 1, fruit))
            .collect::<Vec<_>>()
            .join("\n");

        println!("格式化列表:\n{}", joined);
    }
}

mod advance_collecting {
    // 实现自定义的collect行为
    trait CollectExt<T>: Iterator<Item = T> {
        // 收集到指定容量的Vec
        fn collect_with_capacity(self, capacity: usize) -> Vec<T>
        where
            Self: Sized,
        {
            let mut vec = Vec::with_capacity(capacity);
            for item in self {
                vec.push(item);
            }
            vec
        }

        // 收集并统计
        fn collect_with_stats(self) -> (Vec<T>, usize)
        where
            Self: Sized,
        {
            let mut vec = Vec::new();
            let mut count = 0;

            for item in self {
                vec.push(item);
                count += 1;
            }

            (vec, count)
        }

        // 分批收集
        fn collect_batched(self, batch_size: usize) -> Vec<Vec<T>>
        where
            Self: Sized,
        {
            let mut batches = Vec::new();
            let mut current_batch = Vec::with_capacity(batch_size);

            for item in self {
                current_batch.push(item);
                if current_batch.len() == batch_size {
                    batches.push(std::mem::replace(
                        &mut current_batch,
                        Vec::with_capacity(batch_size),
                    ));
                }
            }

            if !current_batch.is_empty() {
                batches.push(current_batch);
            }

            batches
        }
    }

    impl<T, I: Iterator<Item = T>> CollectExt<T> for I {}

    fn advanced_collect_examples() {
        let data: Vec<i32> = (1..=20).collect();

        // 使用自定义collect方法
        let with_capacity = data.iter().cloned().collect_with_capacity(25);
        println!(
            "预分配容量收集: 长度={}, 容量={}",
            with_capacity.len(),
            with_capacity.capacity()
        );

        let (collected, count) = data.iter().cloned().collect_with_stats();
        println!("带统计收集: 收集了{}个元素", count);

        let batched = data.iter().cloned().collect_batched(5);
        println!("分批收集: {:?}", batched);
    }
}

mod iterator_and_generator {

    #[test]
    // cargo +nightly test --lib -F iterator-patterns -- test_gen_block --nocapture
    fn test_gen_block() {
        let g = gen {
            loop {
                yield 1;
                yield 2;
                return;
            }
        };

        for i in g {
            println!("g: {}", i)
        }
    }

    #[test]
    // cargo +nightly test --lib -F iterator-patterns -- generator_to_iterator --nocapture
    fn generator_to_iterator() {
        // 未来可能的语法
        let fibonacci_gen = gen {
            let (mut a, mut b) = (0, 1);
            loop {
                yield b;
                let temp = a + b;
                a = b;
                b = temp;
            }
        };

        // 作为迭代器使用
        let first_32: Vec<_> = fibonacci_gen.take(32).collect();
        println!("前32个斐波那契数: {:?}", first_32);
    }
}
