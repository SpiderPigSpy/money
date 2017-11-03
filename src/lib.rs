use std::ops::{Add, Mul};
use std::fmt::Debug;

pub trait MonetaryExchange<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> {
    fn exchange(&self, money: Money<V, C>, to_currency: &C) -> Result<Money<V, C>, Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Money<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> {
    pub value: V,
    pub currency: C
}

#[derive(Clone, Debug)]
pub enum Expression<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> {
    Value(Money<V, C>),
    Plus(Box<Expression<V, C>>, Box<Expression<V, C>>),
    Times(Box<Expression<V, C>>, V)
}

#[derive(Clone, Debug, Copy)]
pub enum Error {
    DifferentCurrencies,
    NoExchangeRate
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Expression<V, C> {
    pub fn evaluate<T: MonetaryExchange<V, C>>(self, exchange: &T) -> Result<Money<V, C>, Error> {
        if let Expression::Value(evaluate_result) = self.reduce(exchange)? {
            return Ok(evaluate_result);
        }
        unreachable!();
    }

    fn reduce<T: MonetaryExchange<V, C>>(self, exchange: &T) -> Result<Expression<V, C>, Error> {
        match self {
            Expression::Value(money) => Ok(Expression::Value(money)),
            Expression::Plus(left, right) => {
                let mut left = left.reduce(exchange)?;
                let mut right = right.reduce(exchange)?;
                loop {
                    if let (&Expression::Value(ref left_val), &Expression::Value(ref right_val)) = (&left, &right) {
                        return Ok(Expression::Value(left_val.try_add(right_val, exchange)?.into()));
                    }
                    left = left.reduce(exchange)?;
                    right = right.reduce(exchange)?;
                }
            },
            Expression::Times(expression, multiplier) => {
                let mut expression = expression.reduce(exchange)?;
                loop {
                    if let &Expression::Value(ref value) = &expression {
                        return Ok(Expression::Value(value.clone() * multiplier));
                    }
                    expression = expression.reduce(exchange)?;
                }
            }
        }
    }
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Money<V, C> {
    fn try_add<T: MonetaryExchange<V, C>>(&self, other: &Money<V, C>, exchange: &T) -> Result<Money<V, C>, Error> {
        if self.currency == other.currency {
            Ok(Money{
                value: self.value + other.value,
                currency: self.currency.clone()
            })
        } else {
            exchange.exchange(self.clone(), &other.currency)?.try_add(other, exchange)
        }
    }
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Mul<V> for Money<V, C> {
    type Output = Money<V, C>;

    fn mul(self, other: V) -> Money<V, C> {
        Money {
            value: self.value * other,
            currency: self.currency
        }
    }
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Mul<V> for Expression<V, C> {
    type Output = Expression<V, C>;

    fn mul(self, other: V) -> Expression<V, C> {
        Expression::Times(Box::new(self), other)
    }
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Add for Money<V, C> {
    type Output = Expression<V, C>;

    fn add(self, other: Money<V, C>) -> Expression<V, C> {
        Expression::Plus(Box::new(self.into()), Box::new(other.into()))
    }
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Add<Money<V, C>> for Expression<V, C> {
    type Output = Expression<V, C>;

    fn add(self, other: Money<V, C>) -> Expression<V, C> {
        Expression::Plus(Box::new(self), Box::new(other.into()))
    }
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Add for Expression<V, C> {
    type Output = Expression<V, C>;

    fn add(self, other: Expression<V, C>) -> Expression<V, C> {
        Expression::Plus(Box::new(self), Box::new(other))
    }
}

impl<V: Add<Output=V> + Mul<Output=V> + Copy + Clone + Debug, C: Clone + Debug + Eq + PartialEq> Into<Expression<V, C>> for Money <V, C>{
    fn into(self) -> Expression<V, C> {
        Expression::Value(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    type Value = i64;

    type Currency = String;

    #[allow(dead_code)]
    type Exchange = HashMap<(Currency, Currency), Value>;

    type SimpleMoney = Money<Value, Currency>;

    macro_rules! rates {
        ( $( $from:ident => $to:ident = $value:expr ),* ) => {
            {
                let mut map = HashMap::new();
                $(
                    map.insert((stringify!($from).into(), stringify!($to).into()), $value);
                )*
                map
            }
        };
    }

    #[test]
    fn correctly_evaluates() {
        let rates = rates!{
            USD => RUB = 60, 
            RUB => USD = 0
        };

        assert_eq!((USD(1) + USD(9)).evaluate(&rates).unwrap(), (USD(5) + USD(5)).evaluate(&rates).unwrap());
        assert_eq!(RUB(100), (USD(1) + RUB(40)).evaluate(&rates).unwrap());
        assert_eq!(RUB(100), (RUB(10) * 10 + RUB(0)).evaluate(&rates).unwrap());
    }

    impl MonetaryExchange<Value, Currency> for Exchange {
        fn exchange(&self, money: SimpleMoney, to_currency: &Currency) -> Result<SimpleMoney, Error> {
            if let Some(exchange_rate) = self.get(&(money.currency.clone(), to_currency.clone())) {
                Ok(Money {
                    value: money.value * *exchange_rate,
                    currency: to_currency.clone()
                })
            } else {
                Err(Error::NoExchangeRate)
            }
        }   
    }

    macro_rules! currencies {
        ( $( $cur:ident ),* ) => {
            $(
                #[allow(non_snake_case)]
                fn $cur(value: Value) -> SimpleMoney {
                    Money {
                        value: value,
                        currency: stringify!($cur).into()
                    }
                }
            )*
        };
    }

    currencies! {
        USD, RUB
    }
}
