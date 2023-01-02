use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation};
use std::marker::PhantomData;

//
// selector |       col    |
// ---------+--------------|
//   s0     |      a0      |
//   s1     |      a1      |
//   s2     | a2 = a0 + a1 |
//   s3     | a3 = a1 + a2 |
//
// In this example, we only use one advice column

#[derive(Debug, Clone)]
struct FiboConfig {
    pub advice: Column<Advice>,
    pub instance: Column<Instance>,
    pub selector: Selector,
}

struct FiboChip<F: FieldExt> {
    config: FiboConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> FiboChip<F> {
    pub fn construct(config: FiboConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: Column<Advice>,
        instance: Column<Instance>,
    ) -> FiboConfig {
        let selector = meta.selector();

        // for permutation check
        meta.enable_equality(advice);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            let s = meta.query_selector(selector);
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            let c = meta.query_advice(advice, Rotation(2));
            vec![s * (a + b - c)]
        });

        FiboConfig {
            advice,
            instance,
            selector,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        init_a: Option<F>,
        init_b: Option<F>,
        iter_num: usize,
    ) -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "fibonacci region",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;
                self.config.selector.enable(&mut region, 1)?;

                let mut a = init_a.clone();
                let mut b = init_b.clone();

                region.assign_advice(
                    || "a",
                    self.config.advice,
                    0,
                    || a.ok_or(Error::Synthesis),
                )?;
                let mut b_cell = region.assign_advice(
                    || "b",
                    self.config.advice,
                    1,
                    || b.ok_or(Error::Synthesis),
                )?;

                for row in 2..iter_num {
                    // not to enable selector in the last two rows
                    if row < iter_num - 2 {
                        self.config.selector.enable(&mut region, row)?;
                    }

                    b_cell = region.assign_advice(
                        || "advice",
                        self.config.advice,
                        row,
                        || b.and_then(|b| a.map(|a| a + b)).ok_or(Error::Synthesis),
                    )?;

                    a = b;
                    b = b_cell.value().map(|v| *v);
                }
                Ok(b_cell)
            },
        )
    }

    pub fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        cell: &AssignedCell<F, F>,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
    }
}

#[derive(Default)]
struct MyCircuit<F> {
    pub a: Option<F>,
    pub b: Option<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FiboConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let instance = meta.instance_column();
        FiboChip::configure(meta, advice, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = FiboChip::construct(config);
        let c_cell = chip.assign(layouter.namespace(|| "fibonacci table"), self.a, self.b, 10)?;
        chip.expose_public(layouter.namespace(|| "out"), &c_cell, 0)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MyCircuit;
    use halo2_proofs::{dev::MockProver, pasta::Fp};
    #[test]
    fn test_example2() {
        let k = 2;

        let a = Fp::from(1);
        let b = Fp::from(2);
        let out = Fp::from(89);

        let circuit = MyCircuit {
            a: Some(a),
            b: Some(b),
        };

        let public_input = vec![out];
        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    fn test_example2_failed() {
        let k = 2;

        let a = Fp::from(1);
        let b = Fp::from(2);
        let out = Fp::from(90);

        let circuit = MyCircuit {
            a: Some(a),
            b: Some(b),
        };

        let public_input = vec![out];
        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        // prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_fibonacci2() {
        use plotters::prelude::*;
        let root = BitMapBackend::new("fib-2-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fib 2 Layout", ("sans-serif", 60)).unwrap();

        let a = Fp::from(1);
        let b = Fp::from(1);
        let circuit = MyCircuit {
            a: Some(a),
            b: Some(b),
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }
}
