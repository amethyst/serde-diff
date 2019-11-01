//fn diff<T>(a: &T, b: &T) -> Option<Box<Diff<T>>> {
//    None
//}
//
//fn apply<T>(diff: &dyn Diff<T>, target: &mut T) {
//
//}

trait Diff<T> {
    fn apply(&self, target: &mut T);
}

trait Diffable<T> {
    fn diff(&self, previous: &T) -> Option<Box<dyn Diff<T>>>;
}

struct MyStruct {
    a: f32,
    b: i32,
}

#[derive(Default)]
struct MyStructDiff {
    a: Option<f32>,
    b: Option<i32>,
}

impl Diffable<Self> for MyStruct {
    fn diff(&self, previous: &Self) -> Option<Box<dyn Diff<Self>>> {
        let mut diff = MyStructDiff::default();

        let mut has_change = false;
        if self.a != previous.a {
            diff.a = Some(self.a);
            has_change = true;
        }

        if self.b != previous.b {
            diff.b = Some(self.b);
            has_change = true;
        }

        if has_change {
            Some(Box::new(diff))
        } else {
            None
        }
    }
}

impl Diff<MyStruct> for MyStructDiff {
    fn apply(&self, target: &mut MyStruct) {
        if let Some(a) = self.a {
            target.a = a;
        }

        if let Some(b) = self.b {
            target.b = b;
        }
    }
}

fn main() {}
