use std::path::PathBuf;

use serde::{ Deserialize, Serialize };

use crate::FormatError;

trait Overwrite {
	type Partial;

	fn overwrite(&mut self, other: Self::Partial);
}

macro_rules! identity_overwrite {
    ($($t:ty),*$(,)?) => {
        $(
            impl Overwrite for $t {
                type Partial = Self;
                fn overwrite(&mut self, other: Self) {
                    *self = other;
                }
            }
        )*
    };
}

macro_rules! create_normal_and_partial {
    () => {};
    (struct $name:ident | $partial_name:ident {$(pub $member:ident: $member_type:ty,)*} $($tail:tt)* ) => {
        #[derive(Serialize, Debug)]
        #[serde(rename_all = "kebab-case")]
        pub struct $name {
            $(
                pub $member: $member_type,
            )*
        }

        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "kebab-case")]
        struct $partial_name {
            $(
                pub $member: Option<<$member_type as Overwrite>::Partial>,
            )*
        }

        impl Overwrite for $name {
            type Partial = $partial_name;
            fn overwrite(&mut self, other: $partial_name) {
                $(
                    if let Some(value) = other.$member {
                        self.$member.overwrite(value);
                    }
                )*
            }
        }

        create_normal_and_partial!($($tail)*);
    };
}

// #[derive(Deserialize, Serialize, Debug)]
// #[serde(rename_all = "kebab-case")]
// pub enum UseLongBlock {
//     Never,
//     HasAligment,
//     Always,
// }

// #[derive(Deserialize, Serialize, Debug)]
// #[serde(rename_all = "kebab-case")]
// pub enum LongBlockStyle {
//     Compact,
//     Seperate,
// }

// #[derive(Deserialize, Serialize, Debug)]
// #[serde(rename_all = "kebab-case")]
// pub enum AlignComma {
//     EndOfContent,
//     EndOfCell,
// }

// identity_overwrite!(usize, bool, UseLongBlock, LongBlockStyle, AlignComma);
identity_overwrite!(usize, bool);

create_normal_and_partial!(
    struct ListPaddingSettings | PartialPaddingSettings {
        pub before: bool,
        pub start: bool,
        pub end: bool,
        pub after: bool,
    }

    struct Settings | PartialSettings {
        pub indentation: usize,
        pub final_newline: bool,

        pub use_list: ListPaddingSettings,
        pub parameters: ListPaddingSettings,
        pub arguments: ListPaddingSettings,
    }
);

impl Settings {
	pub fn overwrite(&mut self, path: &PathBuf) -> Result<(), FormatError> {
		let data =
		std::fs::read_to_string(path).map_err(FormatError::FailedToReadConfigurationFile)?;
		let partial = toml::from_str(&data)?;
		<Self as Overwrite>::overwrite(self, partial);
		Ok(())
	}
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			indentation: 0,
			final_newline: true,
			use_list: ListPaddingSettings {
				before: false,
				start: true,
				end: true,
				after: false,
			},
			arguments: ListPaddingSettings {
				before: false,
				start: false,
				end: false,
				after: false,
			},
			parameters: ListPaddingSettings {
				before: false,
				start: false,
				end: false,
				after: false,
			},
		}
	}
}