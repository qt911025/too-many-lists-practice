// 6 不安全单链队列
// 尾部指针直接用了裸指针
// 裸指针不需要指定生命周期

// 对于异步实现，可以用std::pin钉住内存中的某个数据
use std::ptr;

pub struct List<T> {
    head: Link<T>,
    tail: *mut Node<T>, // 裸指针声明不是unsafe的，所以不需要包裹
}

// 为什么要用裸指针？
// 原来的版本是这样的：
/*
pub struct List<'a, T> {
    head: Link<T>,
    tail: Option<&'a mut Node<T>>,
}
*/
// 将引用作为成员时，需要指定生命周期参数
// 因为这里的声明不在一个作用域内，编译器无法算出这个引用的生命周期，所以需要指定。

// 生命周期指定也分声明和使用（形参实参）
// 声明（结构体与函数）时告诉编译器，赋值（结构体）与调用（函数）时，实参的生命周期应该是什么样的
// 实参代入时检查生命周期是否与声明的对齐
// 声明则告知函数体内部每个形参具有什么样的生命周期

// 某些时候为了防止指针悬垂，生命周期标注具有了传染性
// 它会要求有的实现变成类似这样的形式：
/*
pub fn pop(&'a mut self) -> Option<T> {
    self.head.take().map(|head| {
        let head = *head;
        self.head = head.next;

        if self.head.is_none() {
            self.tail = None;
        }

        head.elem
    })
}
*/
// 这会导致在一个作用域内借了就不会还，可变借用用了一次就不能再用。
// 多次pop和push的操作，编译器无法通过借用检查
// 编译器是这么认为的，你以生命周期'a可变借用了self，那么借期就是'a，'a完结借期才完。
// 而'a什么时候完呢？在最后一次使用之后完。
// 那么下次使用的时候当然没完，悖论了。

// 原来没有'a的版本为什么用完就还？
// 因为每次使用都是隐式生成一个独一无二的生命周期

// 借用栈规则（略）
// 用裸指针不要破坏安全引用的借用栈
// type Link<T> = Option<Box<Node<T>>>;

// Box虽然是指针，却拥有所包含对象的所有权，一个拷贝就会重复释放
// 裸指针和Box共同指向同一个内存，Box一释放裸指针就悬垂了
// 而且建立引用还要倒腾几遍解引再引

// 后面可以知道这种写法是invariant的，代表所连接的节点生命周期和指针一样
// 改成covariant的，可以让所连接节点生命周期大于指针
type Link<T> = *mut Node<T>;

struct Node<T> {
    elem: T,
    next: Link<T>,
}

pub struct IntoIter<T>(List<T>);

// 不改成裸指针，因为会丢掉生命周期标识？否则需要用到PhantomData，PhantomData是什么？
pub struct Iter<'a, T> {
    next: Option<&'a Node<T>>,
}

pub struct IterMut<'a, T> {
    next: Option<&'a mut Node<T>>,
}

impl<T> List<T> {
    pub fn new() -> Self {
        List {
            head: ptr::null_mut(),
            tail: ptr::null_mut(), // 裸指针可为null
        }
    }

    // 从尾部入
    pub fn push(&mut self, elem: T) {
        // 创建就转成裸指针了
        // Box本身会被吃掉，但会生成一个裸指针，内部指向的数据也不会释放
        let new_tail = Box::into_raw(
            Box::new(Node {
                elem,
                next: ptr::null_mut(),
            })
        );

        if !self.tail.is_null() {
            unsafe {
                (*self.tail).next = new_tail;
            }
        } else {
            self.head = new_tail;
        }

        self.tail = new_tail;
    }

    // pub fn push(&'a mut self, elem: T) {
    //     let new_tail = Some(
    //         Box::new(Node {
    //             elem,
    //             next: None,
    //         })
    //     );

    //     match self.tail.take() {
    //         Some(old_tail) => {
    //             old_tail.next = new_tail;
    //             self.tail = old_tail.next.as_deref_mut();
    //         }
    //         None => {
    //             self.head = new_tail;
    //             // 关键在于&mut self生命周期是'1，self.head.as_deref_mut()的返回值生命周期也是'1
    //             // 函数结束后，这个生成的引用会归还
    //             // self.tail是将&mut self解引后，打开的一个可以赋值的接口
    //             // &mut self生命周期的消失，和self.tail无关，self.tail的生命周期是和self相同的（成员生命周期与正主等同）
    //             // 如果给&mut self加'a，就意味着声明这个函数执行时，创建的&mut self的生命周期与self等长
    //             self.tail = self.head.as_deref_mut();
    //         }
    //     }

    //     // self.tail = new_tail.as_deref_mut();
    // }

    pub fn pop(&mut self) -> Option<T> {
        if self.head.is_null() {
            None
        } else {
            // Box会重新获得self.head的所有权，之后一定要脱钩，让self.head指向新对象
            let head = unsafe { Box::from_raw(self.head) };
            self.head = head.next;

            if self.head.is_null() {
                self.tail = ptr::null_mut();
            }

            Some(head.elem)
        }
    }

    pub fn peek(&self) -> Option<&T> {
        unsafe { self.head.as_ref().map(|node| &node.elem) }
    }

    // 借用检查呢？
    // 同属一个生命周期的，在最后一次被使用后生命周期结束
    // 在这个函数里，未指明显式的生命周期，将分配一个新生命周期标识符
    // &mut self和&mut T同属一个生命周期，所以，&mut self生命周期结束前，self不可再创建可变引用，
    // 自然也不可再调用这个方法。
    // 直到归还

    // 用借用栈来解释就是，self先压栈，返回值后压栈
    // 再调用peek_mut，将再创建一个新&mut self，使老&mut self及之上都弹栈
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        unsafe { self.head.as_mut().map(|node| &mut node.elem) }
    }
}

impl<T> Drop for List<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
    }
}

//======================================================================

impl<T> IntoIterator for List<T> {
    type Item = T;

    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self)
    }
}

impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            Iter {
                next: self.head.as_ref(),
            }
        }
    }
}

impl<'a, T> IntoIterator for &'a mut List<T> {
    type Item = &'a mut T;

    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            IterMut {
                next: self.head.as_mut(),
            }
        }
    }
}

//====================================================================

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.map(|node| {
            self.next = unsafe { node.next.as_ref() };
            &node.elem
        })
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.next.take().map(|node| {
            self.next = unsafe { node.next.as_mut() };
            &mut node.elem
        })
    }
}

#[cfg(test)]
mod test {
    use super::List;
    #[test]
    fn basics() {
        let mut list = List::new();

        // Check empty list behaves right
        assert_eq!(list.pop(), None);

        // Populate list
        list.push(1);
        list.push(2);
        list.push(3);

        // Check normal removal
        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), Some(2));

        // Push some more just to make sure nothing's corrupted
        list.push(4);
        list.push(5);

        // Check normal removal
        assert_eq!(list.pop(), Some(3));
        assert_eq!(list.pop(), Some(4));

        // Check exhaustion
        assert_eq!(list.pop(), Some(5));
        assert_eq!(list.pop(), None);

        // Check the exhaustion case fixed the pointer right
        list.push(6);
        list.push(7);

        // Check normal removal
        assert_eq!(list.pop(), Some(6));
        assert_eq!(list.pop(), Some(7));
        assert_eq!(list.pop(), None);
    }

    #[test]
    fn into_iter() {
        let mut list = List::new();
        list.push(1);
        list.push(2);
        list.push(3);

        let mut iter = list.into_iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter() {
        let mut list = List::new();
        list.push(1);
        list.push(2);
        list.push(3);

        let mut iter = (&list).into_iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_mut() {
        let mut list = List::new();
        list.push(1);
        list.push(2);
        list.push(3);

        let mut iter = (&mut list).into_iter();
        assert_eq!(iter.next(), Some(&mut 1));
        assert_eq!(iter.next(), Some(&mut 2));
        assert_eq!(iter.next(), Some(&mut 3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn miri_food() {
        // miri是未定义行为检测工具
        let mut list = List::new();

        list.push(1);
        list.push(2);
        list.push(3);

        assert!(list.pop() == Some(1));
        list.push(4);
        assert!(list.pop() == Some(2));
        list.push(5);

        assert!(list.peek() == Some(&3));
        list.push(6);
        list.peek_mut().map(|x| {
            *x *= 10;
        });
        assert!(list.peek() == Some(&30));
        assert!(list.pop() == Some(30));

        for elem in (&mut list).into_iter() {
            *elem *= 100;
        }

        let mut iter = (&list).into_iter();
        assert_eq!(iter.next(), Some(&400));
        assert_eq!(iter.next(), Some(&500));
        assert_eq!(iter.next(), Some(&600));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);

        assert!(list.pop() == Some(400));
        list.peek_mut().map(|x| {
            *x *= 10;
        });
        assert!(list.peek() == Some(&5000));
        list.push(7);

        // Drop it on the ground and let the dtor exercise itself
    }

    // borrow test =====================================================
    // 认识借用栈
    // rust用借用栈来处理再借用
    // 只有栈顶的借用是“正在用的”
    // 访问更低层级的引用会使被访问者变成正在用的，并把更高层的借用全部弹出
    // 已弹出的就不能再访问了
    // 借用检查器保证安全代码遵循以上规则
    // Miri可以在运行时检查裸指针是否遵循以上规则
    #[test]
    fn basic_borrows() {
        // let mut data = 10;
        // let ref1 = &mut data;
        // let ref2 = &mut *ref1;

        // // ORDER SWAPPED!
        // *ref1 += 1;
        // *ref2 += 2;

        // println!("{}", data);

        // 以上违反了借用栈规则

        unsafe {
            // miri会报未定义行为，prt2已弹栈，但还是能运行
            let mut data = 10;
            let ref1 = &mut data;
            let ptr2 = ref1 as *mut _;

            // ORDER SWAPPED!
            *ref1 += 1;
            *ptr2 += 2;

            println!("{}", data);
        }
    }

    #[test]
    fn testing_arrays() {
        unsafe {
            let mut data = [0; 10];
            let ref1_at_0 = &mut data[0]; // Reference to 0th element
            let ptr2_at_0 = ref1_at_0 as *mut i32; // Ptr to 0th element
            let ptr3_at_1 = ptr2_at_0.add(1); // Ptr to 1st element

            *ptr3_at_1 += 3;
            *ptr2_at_0 += 2;
            *ref1_at_0 += 1;

            // Should be [3, 3, 0, ...]
            println!("{:?}", &data[..]);
        }
    }
    // 后面略
    // ============================
}
