use crypto_bigint::{Integer, Uint};
use rand_core::{CryptoRng, OsRng, RngCore};

use crate::hazmat::{
    is_strong_lucas_prime, random_odd_uint, sieve_once, MillerRabin, SelfridgeBase, Sieve,
};

/// Returns a random prime of size `bit_length` using [`OsRng`] as the RNG.
///
/// See [`is_prime_with_rng`] for details about the performed checks.
pub fn prime<const L: usize>(bit_length: usize) -> Uint<L> {
    prime_with_rng(&mut OsRng, bit_length)
}

/// Returns a random safe prime (that is, such that `(n - 1) / 2` is also prime)
/// of size `bit_length` using [`OsRng`] as the RNG.
///
/// See [`is_prime_with_rng`] for details about the performed checks.
pub fn safe_prime<const L: usize>(bit_length: usize) -> Uint<L> {
    safe_prime_with_rng(&mut OsRng, bit_length)
}

/// Checks probabilistically if the given number is prime using [`OsRng`] as the RNG.
///
/// See [`is_prime_with_rng`] for details about the performed checks.
pub fn is_prime<const L: usize>(num: &Uint<L>) -> bool {
    is_prime_with_rng(&mut OsRng, num)
}

/// Checks probabilistically if the given number is a safe prime
/// (that is, such that `(n - 1) / 2` is also prime)
/// using [`OsRng`] as the RNG.
///
/// See [`is_prime_with_rng`] for details about the performed checks.
pub fn is_safe_prime<const L: usize>(num: &Uint<L>) -> bool {
    is_safe_prime_with_rng(&mut OsRng, num)
}

/// Returns a random prime of size `bit_length` using the provided RNG.
///
/// See [`is_prime_with_rng`] for details about the performed checks.
pub fn prime_with_rng<const L: usize>(
    rng: &mut (impl CryptoRng + RngCore),
    bit_length: usize,
) -> Uint<L> {
    loop {
        let start = random_odd_uint::<L, _>(rng, bit_length);
        let sieve = Sieve::new(&start, bit_length);
        for num in sieve {
            if _is_prime_with_rng(rng, &num) {
                return num;
            }
        }
    }
}

/// Returns a random safe prime (that is, such that `(n - 1) / 2` is also prime)
/// of size `bit_length` using the provided RNG.
///
/// See [`is_prime_with_rng`] for details about the performed checks.
pub fn safe_prime_with_rng<const L: usize>(
    rng: &mut (impl CryptoRng + RngCore),
    bit_length: usize,
) -> Uint<L> {
    loop {
        let start = random_odd_uint::<L, _>(rng, bit_length);
        let sieve = Sieve::new(&start, bit_length);
        for num in sieve {
            if _is_safe_prime_with_rng(rng, &num) {
                return num;
            }
        }
    }
}

/// Checks probabilistically if the given number is prime using the provided RNG.
///
/// Performed checks:
/// - Trial division by a number of small primes;
/// - Miller-Rabin check with base 2;
/// - Strong Lucas check with Selfridge base (a.k.a. Baillie method A);
/// - Miller-Rabin check with a random base.
///
/// See [`MillerRabin`] and [`is_strong_lucas_prime`] for more details about the checks.
///
/// The second and the third checks constitute the Baillie-PSW primality test[^Baillie1980];
/// the third one is a precaution that follows the approach of GMP (as of v6.2.1).
/// At the moment of the writing there are no known composites passing
/// the Baillie-PSW test[^Baillie2021];
/// it is conjectured that they may exist, but their size is larger than the numbers
/// that are used in practice.
///
/// [^Baillie1980]: R. Baillie, S. S. Wagstaff, "Lucas pseudoprimes",
///       Math. Comp. 35 1391-1417 (1980),
///       DOI: [10.2307/2006406](https://dx.doi.org/10.2307/2006406),
///       <http://mpqs.free.fr/LucasPseudoprimes.pdf>
///
/// [^Baillie2021]: R. Baillie, A. Fiori, S. S. Wagstaff,
///       "Strengthening the Baillie-PSW primality test",
///       Math. Comp. 90 1931-1955 (2021),
///       DOI: [10.1090/mcom/3616](https://doi.org/10.1090/mcom/3616)
pub fn is_prime_with_rng<const L: usize>(
    rng: &mut (impl CryptoRng + RngCore),
    num: &Uint<L>,
) -> bool {
    if let Some(is_prime) = sieve_once(num) {
        return is_prime;
    }
    _is_prime_with_rng(rng, num)
}

/// Checks probabilistically if the given number is prime using the provided RNG.
///
/// See [`is_prime_with_rng`] for details about the performed checks.
pub fn is_safe_prime_with_rng<const L: usize>(
    rng: &mut (impl CryptoRng + RngCore),
    num: &Uint<L>,
) -> bool {
    if let Some(is_prime) = sieve_once(num) {
        return is_prime;
    }
    _is_safe_prime_with_rng(rng, num)
}

/// Checks for safe prime assuming that `num` was already pre-sieved.
fn _is_safe_prime_with_rng<const L: usize>(
    rng: &mut (impl CryptoRng + RngCore),
    num: &Uint<L>,
) -> bool {
    debug_assert!(bool::from(num.is_odd()));
    if !_is_prime_with_rng(rng, num) {
        return false;
    }
    if !is_prime_with_rng(rng, &(num >> 1)) {
        return false;
    }
    true
}

/// Checks for primality assuming that `num` was already pre-sieved.
fn _is_prime_with_rng<const L: usize>(rng: &mut (impl CryptoRng + RngCore), num: &Uint<L>) -> bool {
    debug_assert!(bool::from(num.is_odd()));
    let mr = MillerRabin::new(num);
    if !mr.check_base_two() {
        return false;
    }
    if !is_strong_lucas_prime(num, SelfridgeBase, true) {
        return false;
    }
    if !mr.check_random_base(rng) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use crypto_bigint::{CheckedAdd, Uint, U128, U64};

    use super::{is_prime, is_safe_prime, prime, safe_prime};
    use crate::hazmat::{primes, pseudoprimes};

    fn test_large_primes<const L: usize>(nums: &[Uint<L>]) {
        for num in nums {
            assert!(is_prime(num));
        }
    }

    #[test]
    fn large_primes() {
        test_large_primes(primes::PRIMES_128);
        test_large_primes(primes::PRIMES_256);
        test_large_primes(primes::PRIMES_384);
        test_large_primes(primes::PRIMES_512);
        test_large_primes(primes::PRIMES_1024);
    }

    fn test_pseudoprimes(nums: &[u32]) {
        for num in nums {
            assert!(!is_prime(&U64::from(*num)));
        }
    }

    #[test]
    fn pseudoprimes() {
        test_pseudoprimes(pseudoprimes::EXTRA_STRONG_LUCAS);
        test_pseudoprimes(pseudoprimes::STRONG_LUCAS);
        test_pseudoprimes(pseudoprimes::ALMOST_EXTRA_STRONG_LUCAS);
        test_pseudoprimes(pseudoprimes::STRONG_BASE_2);
        test_pseudoprimes(pseudoprimes::FIBONACCI);
        test_pseudoprimes(pseudoprimes::BRUCKMAN_LUCAS);
        test_pseudoprimes(pseudoprimes::LUCAS);

        for num in pseudoprimes::STRONG_FIBONACCI {
            assert!(!is_prime(num));
        }

        assert!(!is_prime(&pseudoprimes::LARGE_CARMICHAEL_NUMBER));
    }

    fn test_cunningham_chain<const L: usize>(length: usize, num: &Uint<L>) {
        let mut next = *num;
        for i in 0..length {
            assert!(is_prime(&next));

            // The start of the chain isn't a safe prime by definition
            if i > 0 {
                assert!(is_safe_prime(&next));
            }

            next = (next << 1).checked_add(&Uint::<L>::ONE).unwrap();
        }

        // The chain ended.
        assert!(!is_prime(&next));
    }

    #[test]
    fn cunningham_chains() {
        for (length, num) in primes::CUNNINGHAM_CHAINS_128 {
            test_cunningham_chain(*length, num);
        }
        for (length, num) in primes::CUNNINGHAM_CHAINS_512 {
            test_cunningham_chain(*length, num);
        }
    }

    #[test]
    fn generate_prime() {
        for bit_length in (28..=128).step_by(10) {
            let p: U128 = prime(bit_length);
            assert!(p.bits_vartime() == bit_length);
            assert!(is_prime(&p));
        }
    }

    #[test]
    fn generate_safe_prime() {
        for bit_length in (28..=128).step_by(10) {
            let p: U128 = safe_prime(bit_length);
            assert!(p.bits_vartime() == bit_length);
            assert!(is_safe_prime(&p));
        }
    }
}

#[cfg(feature = "tests-openssl")]
mod tests_openssl {
    use crypto_bigint::U128;
    use openssl::bn::{BigNum, BigNumContext};
    use rand_core::OsRng;

    use super::{is_prime, prime};
    use crate::hazmat::random_odd_uint;

    fn openssl_is_prime(num: &BigNum, ctx: &mut BigNumContext) -> bool {
        num.is_prime(64, ctx).unwrap()
    }

    fn to_openssl(num: &U128) -> BigNum {
        BigNum::from_hex_str(&format!("{:x}", num)).unwrap()
    }

    fn from_openssl(num: &BigNum) -> U128 {
        U128::from_be_hex(&num.to_hex_str().unwrap())
    }

    #[test]
    fn openssl_cross_check() {
        let mut ctx = BigNumContext::new().unwrap();

        // Generate primes, let OpenSSL check them
        for _ in 0..100 {
            let p: U128 = prime(128);
            let p_bn = to_openssl(&p);
            assert!(
                openssl_is_prime(&p_bn, &mut ctx),
                "OpenSSL reports {} as composite",
                p
            );
        }

        // Generate primes with OpenSSL, check them
        let mut p_bn = BigNum::new().unwrap();
        for _ in 0..100 {
            p_bn.generate_prime(128, false, None, None).unwrap();
            let p = from_openssl(&p_bn);
            assert!(is_prime(&p), "we report {} as composite", p);
        }

        // Generate random numbers, check if our test agrees with OpenSSL
        for _ in 0..100 {
            let p: U128 = random_odd_uint(&mut OsRng, 128);
            let actual = is_prime(&p);
            let p_bn = to_openssl(&p);
            let expected = openssl_is_prime(&p_bn, &mut ctx);
            assert_eq!(
                actual, expected,
                "difference between OpenSSL and us: OpenSSL reports {}, we report {}",
                expected, actual
            );
        }
    }
}

#[cfg(feature = "tests-gmp")]
mod tests_gmp {
    use crypto_bigint::U128;
    use rand_core::OsRng;
    use rug::{
        integer::{IsPrime, Order},
        Integer,
    };

    use super::{is_prime, prime};
    use crate::hazmat::random_odd_uint;

    fn gmp_is_prime(num: &Integer) -> bool {
        matches!(num.is_probably_prime(25), IsPrime::Yes | IsPrime::Probably)
    }

    fn to_gmp(num: &U128) -> Integer {
        Integer::from_digits(num.as_words(), Order::Lsf)
    }

    fn from_gmp(num: &Integer) -> U128 {
        U128::from_words(num.to_digits(Order::Lsf).try_into().unwrap())
    }

    #[test]
    fn gmp_cross_check() {
        // Generate primes, let GMP check them
        for _ in 0..100 {
            let p: U128 = prime(128);
            let p_bn = to_gmp(&p);
            assert!(gmp_is_prime(&p_bn), "GMP reports {} as composite", p);
        }

        // Generate primes with GMP, check them
        for _ in 0..100 {
            let start: U128 = random_odd_uint(&mut OsRng, 128);
            let start_bn = to_gmp(&start);
            let p_bn = start_bn.next_prime();
            let p = from_gmp(&p_bn);
            assert!(is_prime(&p), "we report {} as composite", p);
        }

        // Generate random numbers, check if our test agrees with GMP
        for _ in 0..100 {
            let p: U128 = random_odd_uint(&mut OsRng, 128);
            let actual = is_prime(&p);
            let p_bn = to_gmp(&p);
            let expected = gmp_is_prime(&p_bn);
            assert_eq!(
                actual, expected,
                "difference between GMP and us: GMP reports {}, we report {}",
                expected, actual
            );
        }
    }
}
