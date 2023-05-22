use concat_idents::concat_idents;

#[macro_export]
macro_rules! ubo_struct {
    ($var:ident, $ifc:ident, struct $name:ident {$($fname:ident:$ftype:ty$(,)*),*}) => {
        concat_idents::concat_idents!(buffer_name = $var, _, buffer {
            #[repr(C)]
            struct $name {
                $($fname:$ftype,)*
            }

            let mut buffer_name = $ifc.allocate_scratch_ubo(std::mem::size_of::<$name>() as vk::DeviceSize)?;
            let $var = buffer_name.mapped_slice::<$name>()?;
            let mut $var = $var.get_mut(0).unwrap();
        });
    };
}
