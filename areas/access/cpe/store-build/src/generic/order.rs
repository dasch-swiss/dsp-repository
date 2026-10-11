//! The positional order of a compound's members.

/// The part of an IRI after its last `/`; the whole IRI where it has none.
pub(crate) fn iri_local(iri: &str) -> &str {
    iri.rsplit_once('/').map_or(iri, |(_, local)| local)
}

/// Sorts `members` into positional order: by `seqnum`, then by the local part of the IRI. The
/// archive's `seqnum` has ties, and the local part is what breaks them. The whole IRI is the last
/// key, so that the order is total.
///
/// `key` gives a member's `seqnum` and IRI.
pub(crate) fn sort_positional<T>(members: &mut [T], key: impl Fn(&T) -> (i64, &str)) {
    members.sort_by(|a, b| {
        let (a_seqnum, a_iri) = key(a);
        let (b_seqnum, b_iri) = key(b);
        (a_seqnum, iri_local(a_iri), a_iri).cmp(&(b_seqnum, iri_local(b_iri), b_iri))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(mut members: Vec<(i64, &str)>) -> Vec<(i64, &str)> {
        sort_positional(&mut members, |&(seqnum, iri)| (seqnum, iri));
        members
    }

    #[test]
    fn test_iri_local_is_the_part_after_the_last_slash() {
        assert_eq!(iri_local("http://rdfh.ch/0803/abc"), "abc");
        assert_eq!(iri_local("abc"), "abc");
        assert_eq!(iri_local("http://rdfh.ch/0803/"), "");
    }

    #[test]
    fn test_sort_positional_orders_by_seqnum_before_the_iri() {
        assert_eq!(
            sorted(vec![
                (2, "http://rdfh.ch/0803/a"),
                (10, "http://rdfh.ch/0803/b"),
                (1, "http://rdfh.ch/0803/z"),
            ]),
            [
                (1, "http://rdfh.ch/0803/z"),
                (2, "http://rdfh.ch/0803/a"),
                (10, "http://rdfh.ch/0803/b"),
            ]
        );
    }

    #[test]
    fn test_sort_positional_equal_seqnum_orders_by_local_part_not_by_the_whole_iri() {
        assert_eq!(
            sorted(vec![(1, "http://b.example/x/m"), (1, "http://a.example/x/n"),]),
            [(1, "http://b.example/x/m"), (1, "http://a.example/x/n")]
        );
    }

    #[test]
    fn test_sort_positional_equal_seqnum_and_local_part_orders_by_the_whole_iri() {
        assert_eq!(
            sorted(vec![(1, "http://b.example/x/m"), (1, "http://a.example/x/m"),]),
            [(1, "http://a.example/x/m"), (1, "http://b.example/x/m")]
        );
    }
}
