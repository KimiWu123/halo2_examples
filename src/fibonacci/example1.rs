use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation};
use std::marker::PhantomData;

//
// selector | col_a | col_b | col_c
// ---------+-------+-------+-------
//   s0     |   a0  |   b0  |   c0
//   s1     |   a1  |   b1  |   c1
//
// here, we copy the values from previous row(b and c) to the next row(a and b)
// ==> a1 = b0, b1 = c0
// So, we need to turn on permutation check on a, b and c

#[derive(Debug, Clone)]
struct FiboConfig {
    pub advice: [Column<Advice>; 3],
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
        advice: [Column<Advice>; 3],
        instance: Column<Instance>,
    ) -> FiboConfig {
        let col_a = advice[0];
        let col_b = advice[1];
        let col_c = advice[2];
        let selector = meta.selector();

        // for permutation check
        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            let s = meta.query_selector(selector);
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());
            vec![s * (a + b - c)]
        });

        FiboConfig {
            advice: [col_a, col_b, col_c],
            instance,
            selector,
        }
    }

    pub fn assign_row(
        &self,
        mut layouter: impl Layouter<F>,
        prev_b: Option<F>,
        prev_c: Option<F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        // selector | col_a | col_b | col_c
        // ---------+-------+-------+-------
        //   s1     |   b0  |   c0  | b0 + c0 = c1

        layouter.assign_region(
            || "row",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;
                let c_val = prev_b.and_then(|b| prev_c.map(|c| b + c));

                region.assign_advice(
                    || "a",
                    self.config.advice[0],
                    0,
                    || prev_b.ok_or(Error::Synthesis),
                )?;
                region.assign_advice(
                    || "b",
                    self.config.advice[1],
                    0,
                    || prev_c.ok_or(Error::Synthesis),
                )?;
                let c_cell = region.assign_advice(
                    || "c",
                    self.config.advice[2],
                    0,
                    || c_val.ok_or(Error::Synthesis),
                )?;

                Ok(c_cell)
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
        let col_a = meta.advice_column();
        let col_b = meta.advice_column();
        let col_c = meta.advice_column();
        let instance = meta.instance_column();

        FiboChip::configure(meta, [col_a, col_b, col_c], instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = FiboChip::construct(config);

        let mut prev_b = self.a;
        let mut prev_c = self.b;
        let mut c_cell = chip.assign_row(layouter.namespace(|| "next row"), prev_b, prev_c)?;
        for _i in 3..10 {
            prev_b = prev_c;
            prev_c = c_cell.value().map(|v| *v);
            c_cell = chip.assign_row(layouter.namespace(|| "next row"), prev_b, prev_c)?;
        }

        chip.expose_public(layouter.namespace(|| "out"), &c_cell, 0)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MyCircuit;
    use halo2_proofs::{dev::MockProver, pasta::Fp};
    #[test]
    fn test_example1() {
        let k = 4;

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
    fn test_example1_failed() {
        let k = 4;

        let a = Fp::from(1);
        let b = Fp::from(2);
        let out = Fp::from(90);

        let circuit = MyCircuit {
            a: Some(a),
            b: Some(b),
        };

        let public_input = vec![out];
        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_fibonacci1() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("fib--layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fib 1 Layout", ("sans-serif", 60)).unwrap();

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
