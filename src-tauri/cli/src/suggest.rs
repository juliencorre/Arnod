//! Proposition la plus proche d'une saisie inconnue (LC5) : cible, environnement, option.

/// Distance d'édition (Levenshtein) entre deux textes, sans casse.
pub fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.to_lowercase().chars().collect();
    let b: Vec<char> = b.to_lowercase().chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let value = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
            current.push(value);
        }
        previous = current;
    }
    previous[b.len()]
}

/// Candidat le plus proche, s'il l'est assez (au plus un tiers de la saisie à changer, au moins
/// deux caractères).
pub fn closest<'a>(input: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let limit = (input.chars().count() / 3).max(2);
    candidates
        .map(|candidate| (distance(input, candidate), candidate))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, candidate)| (*d, candidate.len()))
        .map(|(_, candidate)| candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lc5_la_plus_proche_si_elle_l_est_assez() {
        assert_eq!(distance("commande/v2", "commandes/v2"), 1);
        assert_eq!(distance("Recette", "recette"), 0);
        let targets = ["commandes", "commandes/v2", "paiements"];
        assert_eq!(
            closest("commande/v2", targets.into_iter()),
            Some("commandes/v2")
        );
        assert_eq!(closest("paiement", targets.into_iter()), Some("paiements"));
        assert_eq!(closest("clients", targets.into_iter()), None);
    }
}
