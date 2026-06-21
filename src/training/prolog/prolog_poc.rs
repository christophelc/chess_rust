use anyhow::{anyhow, Result};
use scryer_prolog::{MachineBuilder, Term /* Machine, QueryState, etc. */};
use std::{collections::BTreeMap, fs};

fn prolog_program() -> std::io::Result<String> {
    let content = fs::read_to_string("prolog/training/poc.pl")?;
    Ok(content)
}

fn term_to_vec_i64(t: &Term) -> Result<Vec<i64>> {
    match t {
        Term::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                match it {
                    Term::Integer(big) => out.push(big.to_string().parse::<i64>()?),
                    other => return Err(anyhow!("élément non entier dans la liste: {other:?}")),
                }
            }
            Ok(out)
        }
        other => Err(anyhow!("le terme n'est pas une liste: {other:?}")),
    }
}

fn run_n_consecutifs(n: i64, start: i64, program: &str) -> Result<Vec<i64>> {
    let mut machine = MachineBuilder::new().build(); // in-memory streams par défaut
    machine.consult_module_string("user", program);

    // Get iterator
    let q = format!("n_consecutifs({}, {}, L).", n, start);
    let mut answers = machine.run_query(q);

    // Iterate
    let first = answers
        .next()
        .ok_or_else(|| anyhow!("aucune réponse"))? // Result<LeafAnswer, Term>
        .map_err(|e| anyhow!("erreur Prolog: {e:?}"))?; // Result<_, Term> -> map Term to anyhow::Error

    // LeafAnswer::LeafAnswer { bindings, .. }
    let bindings: &BTreeMap<String, Term> = match first {
        scryer_prolog::LeafAnswer::LeafAnswer { ref bindings, .. } => bindings,
        scryer_prolog::LeafAnswer::True => return Err(anyhow!("réponse sans variables (True)")),
        scryer_prolog::LeafAnswer::False => return Err(anyhow!("échec de la requête (False)")),
        scryer_prolog::LeafAnswer::Exception(ref term) => {
            return Err(anyhow!("exception Prolog: {term:?}"))
        }
    };

    let l_term = bindings
        .get("L")
        .ok_or_else(|| anyhow!("variable L non liée"))?;

    term_to_vec_i64(l_term)
}

mod tests {

    use super::*;

    fn read_prolog_program() -> String {
        prolog_program().expect("Erreur when reading Prolog program")
    }

    #[ignore]
    fn test_n_consecutifs_basique() {
        let result =
            run_n_consecutifs(5, 10, &read_prolog_program()).expect("Erreur d'exécution Prolog");
        assert_eq!(result, vec![10, 11, 12, 13, 14]);
    }

    #[ignore]
    fn test_n_consecutifs_zero() {
        let result =
            run_n_consecutifs(0, 42, &read_prolog_program()).expect("Erreur d'exécution Prolog");
        assert_eq!(result, Vec::<i64>::new());
    }

    #[ignore]
    fn test_n_consecutifs_un() {
        let result =
            run_n_consecutifs(1, -3, &read_prolog_program()).expect("Erreur d'exécution Prolog");
        assert_eq!(result, vec![-3]);
    }
}
