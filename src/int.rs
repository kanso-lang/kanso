//! The interpreter's integer. Kanso's ints are unbounded, but nearly every
//! one a program makes fits a machine word, and a `BigInt` pays a heap
//! allocation for its digits even when it holds 1. So a value that fits is
//! kept as an `i64`, and only one that overflows takes the `BigInt`, shared
//! behind an `Rc` so that copying the value never copies the digits.
//!
//! The two forms are kept canonical: `Big` never holds a number that fits
//! `Small`. Equality, ordering and hashing rely on that, since two equal
//! numbers are then always the same variant.

use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Int {
    Small(i64),
    Big(Rc<BigInt>),
}

impl Int {
    /// The digits as a `BigInt`, borrowed when they already are one.
    pub fn big(&self) -> Cow<'_, BigInt> {
        match self {
            Int::Small(n) => Cow::Owned(BigInt::from(*n)),
            Int::Big(n) => Cow::Borrowed(n),
        }
    }

    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Int::Small(n) => Some(*n),
            Int::Big(_) => None,
        }
    }

    pub fn to_u64(&self) -> Option<u64> {
        u64::try_from(self).ok()
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Int::Small(0))
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Int::Small(n) => *n as f64,
            Int::Big(n) => n.to_f64().unwrap_or(f64::INFINITY),
        }
    }

    pub fn add(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_add(*b) {
                Some(n) => Int::Small(n),
                None => Int::from(BigInt::from(*a) + b),
            },
            _ => Int::from(self.big().as_ref() + other.big().as_ref()),
        }
    }

    pub fn sub(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_sub(*b) {
                Some(n) => Int::Small(n),
                None => Int::from(BigInt::from(*a) - b),
            },
            _ => Int::from(self.big().as_ref() - other.big().as_ref()),
        }
    }

    pub fn mul(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_mul(*b) {
                Some(n) => Int::Small(n),
                None => Int::from(BigInt::from(*a) * b),
            },
            _ => Int::from(self.big().as_ref() * other.big().as_ref()),
        }
    }

    /// Truncating division, as `BigInt`'s is. The divisor is not zero.
    pub fn div(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_div(*b) {
                Some(n) => Int::Small(n),
                None => Int::from(BigInt::from(*a) / b),
            },
            _ => Int::from(self.big().as_ref() / other.big().as_ref()),
        }
    }

    /// The remainder of truncating division. The divisor is not zero.
    pub fn rem(&self, other: &Int) -> Int {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => match a.checked_rem(*b) {
                Some(n) => Int::Small(n),
                None => Int::from(BigInt::from(*a) % b),
            },
            _ => Int::from(self.big().as_ref() % other.big().as_ref()),
        }
    }
}

/// The number as a word, if it fits one. `BigInt::to_i64` gets there through
/// a general digit walk; a number that fits has at most one digit, so this
/// reads it directly.
#[inline]
fn word_of(n: &BigInt) -> Option<i64> {
    let mut digits = n.iter_u64_digits();
    let magnitude = match digits.len() {
        0 => return Some(0),
        1 => digits.next()?,
        _ => return None,
    };
    match n.sign() {
        Sign::Minus if magnitude <= 1 << 63 => Some((magnitude as i64).wrapping_neg()),
        Sign::Minus => None,
        _ => i64::try_from(magnitude).ok(),
    }
}

impl From<BigInt> for Int {
    fn from(n: BigInt) -> Int {
        // An arithmetic result that fits a word goes back into one.
        match word_of(&n) {
            Some(small) => Int::Small(small),
            None => Int::Big(Rc::new(n)),
        }
    }
}

impl From<&BigInt> for Int {
    // Inlined into the literal's evaluation, which almost never takes the
    // clone; kept out of line, the clone's register saves were paid by every
    // literal.
    #[inline]
    fn from(n: &BigInt) -> Int {
        match word_of(n) {
            Some(small) => Int::Small(small),
            None => wide(n),
        }
    }
}

#[cold]
#[inline(never)]
fn wide(n: &BigInt) -> Int {
    Int::Big(Rc::new(n.clone()))
}

macro_rules! from_word {
    ($($t:ty),*) => {$(
        impl From<$t> for Int {
            fn from(n: $t) -> Int {
                Int::Small(i64::from(n))
            }
        }
    )*};
}
from_word!(i8, i16, i32, i64, u8, u16, u32);

macro_rules! from_wide {
    ($($t:ty),*) => {$(
        impl From<$t> for Int {
            fn from(n: $t) -> Int {
                match i64::try_from(n) {
                    Ok(small) => Int::Small(small),
                    Err(_) => Int::Big(Rc::new(BigInt::from(n))),
                }
            }
        }
    )*};
}
from_wide!(u64, usize, i128, u128);

macro_rules! narrow_to {
    ($($t:ty),*) => {$(
        impl TryFrom<&Int> for $t {
            type Error = ();
            fn try_from(n: &Int) -> Result<$t, ()> {
                match n {
                    Int::Small(v) => <$t>::try_from(*v).map_err(|_| ()),
                    Int::Big(_) => Err(()),
                }
            }
        }
    )*};
}
narrow_to!(u8, u32, i32, u64, usize);

impl Ord for Int {
    fn cmp(&self, other: &Int) -> Ordering {
        match (self, other) {
            (Int::Small(a), Int::Small(b)) => a.cmp(b),
            _ => self.big().as_ref().cmp(other.big().as_ref()),
        }
    }
}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Int) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq<BigInt> for Int {
    fn eq(&self, other: &BigInt) -> bool {
        match self {
            Int::Small(n) => word_of(other) == Some(*n),
            Int::Big(n) => n.as_ref() == other,
        }
    }
}

impl fmt::Display for Int {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Int::Small(n) => write!(f, "{n}"),
            Int::Big(n) => write!(f, "{n}"),
        }
    }
}

impl Zero for Int {
    fn zero() -> Int {
        Int::Small(0)
    }
    fn is_zero(&self) -> bool {
        Int::is_zero(self)
    }
}

impl std::ops::Add for Int {
    type Output = Int;
    fn add(self, other: Int) -> Int {
        Int::add(&self, &other)
    }
}
