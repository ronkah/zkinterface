use std::io::Write;
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use serde::{Deserialize, Serialize};
use crate::Result;
use crate::zkinterface_generated::zkinterface::{GatesSystem, GatesSystemArgs, Message, Root, RootArgs};
use super::gates::GateOwned;


#[derive(Clone, Default, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GatesSystemOwned {
    pub gates: Vec<GateOwned>,
}

impl<'a> From<GatesSystem<'a>> for GatesSystemOwned {
    /// Convert from Flatbuffers references to owned structure.
    fn from(system_ref: GatesSystem) -> GatesSystemOwned {
        let mut owned = GatesSystemOwned {
            gates: vec![],
        };

        let gates_ref = system_ref.gates().unwrap();
        for i in 0..gates_ref.len() {
            let gate_ref = gates_ref.get(i);
            owned.gates.push(GateOwned::from(gate_ref));
        }

        owned
    }
}

impl GatesSystemOwned {
    /// Add this structure into a Flatbuffers message builder.
    pub fn build<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        &'args self,
        builder: &'mut_bldr mut FlatBufferBuilder<'bldr>,
    ) -> WIPOffset<Root<'bldr>>
    {
        let gates: Vec<_> = self.gates.iter()
            .map(|gate|
                gate.build(builder)
            ).collect();

        let gates = builder.create_vector(&gates);
        let gates_system = GatesSystem::create(builder, &GatesSystemArgs {
            gates: Some(gates)
        });

        Root::create(builder, &RootArgs {
            message_type: Message::GatesSystem,
            message: Some(gates_system.as_union_value()),
        })
    }

    /// Writes this constraint system as a Flatbuffers message into the provided buffer.
    ///
    /// # Examples
    /// ```
    /// let mut buf = Vec::<u8>::new();
    /// let gate_system = zkinterface::GatesSystemOwned { gates: vec![] };
    /// gate_system.write_into(&mut buf).unwrap();
    /// assert!(buf.len() > 0);
    /// ```
    pub fn write_into(&self, writer: &mut impl Write) -> Result<()> {
        let mut builder = FlatBufferBuilder::new();
        let message = self.build(&mut builder);
        builder.finish_size_prefixed(message, None);
        writer.write_all(builder.finished_data())?;
        Ok(())
    }
}
