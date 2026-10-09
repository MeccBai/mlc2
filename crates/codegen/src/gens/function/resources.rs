use super::*;

impl FunctionGenerator<'_> {
    pub(super) fn drop_owned(&mut self, ty: TypeIndex, storage: &LlvmValue) {
        IrGenerator::append_drop(
            ty,
            storage.reg.unwrap(),
            &mut self.next_reg,
            &mut self.code,
            self.package,
        );
    }
}
