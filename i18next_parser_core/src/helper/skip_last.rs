use std::iter::Peekable;

pub(crate) struct SkipLastIterator<I: Iterator>(Peekable<I>);
impl<I: Iterator> Iterator for SkipLastIterator<I> {
  type Item = I::Item;

  fn next(&mut self) -> Option<Self::Item> {
    let item = self.0.next();
    self.0.peek().map(|_| item.unwrap())
  }
}

pub(crate) trait SkipLast: Iterator + Sized {
  fn skip_last(self) -> SkipLastIterator<Self> {
    SkipLastIterator(self.peekable())
  }
}
impl<I: Iterator> SkipLast for I {
}
