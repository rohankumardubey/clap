// @TODO @p2 @docs remove Arg::setting(foo) in examples, we are sticking with Arg::foo(true) instead
mod settings;
mod key;
mod value;
mod validation_rules;
mod occurrence;
mod help_message;
mod display_order;

use std::borrow::Cow;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display, Formatter};
use std::hash::Hash;
#[cfg(not(any(target_os = "windows", target_arch = "wasm32")))]
use std::os::unix::ffi::OsStrExt;
use std::rc::Rc;
use std::str;

#[cfg(feature = "yaml")]
use yaml_rust;

use crate::build::UsageParser;
use crate::INTERNAL_ERROR_MSG;
#[cfg(any(target_os = "windows", target_arch = "wasm32"))]
use crate::osstringext::OsStrExt3;
use crate::util::hash;
use crate::util::VecMap;

pub use self::key::{Key, Position, Short, Long};
pub use self::settings::{ArgFlags, ArgSettings};
pub use self::occurrence::Occurrence;

pub type ArgId = u64;

/// The abstract representation of a command line argument. Used to set all the options and
/// relationships that define a valid argument for the program.
///
/// There are two methods for constructing [`Arg`]s, using the builder pattern and setting options
/// manually, or using a usage string which is far less verbose but has fewer options. You can also
/// use a combination of the two methods to achieve the best of both worlds.
///
/// # Examples
///
/// ```rust
/// # use clap::Arg;
/// // Using the traditional builder pattern and setting each option manually
/// let cfg = Arg::new("config")
///       .short('c')
///       .long("config")
///       .takes_value(true)
///       .value_name("FILE")
///       .help("Provides a config file to myprog");
/// // Using a usage string (setting a similar argument to the one above)
/// let input = Arg::from("-i, --input=[FILE] 'Provides an input file to the program'");
/// ```
/// [`Arg`]: ./struct.Arg.html
#[allow(missing_debug_implementations)]
#[derive(Default, Clone)]
pub struct Arg<'help> {
    #[doc(hidden)]
    pub id: ArgId,
    #[doc(hidden)]
    key: Key<'help>,
    #[doc(hidden)]
    settings: ArgFlags,
    value: Option<Value>,
    help: HelpMessage,
    occurrence: Occurrence,
    validation: ValidationRules<'help>,

}

impl<'help> Arg<'help> {
    /// Creates a new instance of [`Arg`] using a unique string name. The name will be used to get
    /// information about whether or not the argument was used at runtime, get values, set
    /// relationships with other args, etc..
    ///
    /// **NOTE:** In the case of arguments that take values (i.e. [`Arg::takes_value(true)`])
    /// and positional arguments (i.e. those without a preceding `-` or `--`) the name will also
    /// be displayed when the user prints the usage/help information of the program.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("config")
    /// # ;
    /// ```
    /// [`Arg::takes_value(true)`]: ./struct.Arg.html#method.takes_value
    /// [`Arg`]: ./struct.Arg.html
    pub fn new<T>(id: T) -> Self where T: Hash{
        Arg {
            id: hash(id),
            settings: ArgFlags::default(),
            key: Key::new(),
            value: None,
            help: HelpMessage::new(),
            occurrence: Occurrence::new(),
            validation: ValidationRules::new(),
        }
    }

    /// Sets the short version of the argument without the preceding `-`.
    ///
    /// By default `clap` automatically assigns `V` and `h` to the auto-generated `version` and
    /// `help` arguments respectively. You may use the uppercase `V` or lowercase `h` for your own
    /// arguments, in which case `clap` simply will not assign those to the auto-generated
    /// `version` or `help` arguments.
    ///
    /// # Examples
    ///
    /// To set [`short`] use a single valid UTF-8 character. If you supply a leading `-` such as
    /// `-c`, the `-` will be stripped.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("config")
    ///     .short('c')
    /// # ;
    /// ```
    ///
    /// Setting [`short`] allows using the argument via a single hyphen (`-`) such as `-c`
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("config")
    ///         .short('c'))
    ///     .get_matches_from(vec![
    ///         "prog", "-c"
    ///     ]);
    ///
    /// assert!(m.is_present("config"));
    /// ```
    /// [`short`]: ./struct.Arg.html#method.short
    pub fn short(mut self, s: char) -> Self {
        self.key.short(s);
        self
    }

    /// Sets the long version of the argument without the preceding `--`.
    ///
    /// By default `clap` automatically assigns `version` and `help` to the auto-generated
    /// `version` and `help` arguments respectively. You may use the word `version` or `help` for
    /// the long form of your own arguments, in which case `clap` simply will not assign those to
    /// the auto-generated `version` or `help` arguments.
    ///
    /// **NOTE:** Any leading `-` characters will be stripped
    ///
    /// # Examples
    ///
    /// To set `long` use a word containing valid UTF-8 codepoints. If you supply a double leading
    /// `--` such as `--config` they will be stripped. Hyphens in the middle of the word, however,
    /// will *not* be stripped (i.e. `config-file` is allowed)
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("cfg")
    ///     .long("config")
    /// # ;
    /// ```
    ///
    /// Setting `long` allows using the argument via a double hyphen (`--`) such as `--config`
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config"))
    ///     .get_matches_from(vec![
    ///         "prog", "--config"
    ///     ]);
    ///
    /// assert!(m.is_present("cfg"));
    /// ```
    pub fn long(mut self, l: &'help str) -> Self {
        self.key.long(l);
        self
    }

    /// Allows adding a [`Arg`] alias, which function as "hidden" arguments that
    /// automatically dispatch as if this argument was used. This is more efficient, and easier
    /// than creating multiple hidden arguments as one only needs to check for the existence of
    /// this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///             .arg(Arg::new("test")
    ///             .long("test")
    ///             .alias("alias")
    ///             .takes_value(true))
    ///        .get_matches_from(vec![
    ///             "prog", "--alias", "cool"
    ///         ]);
    /// assert!(m.is_present("test"));
    /// assert_eq!(m.value_of("test"), Some("cool"));
    /// ```
    /// [`Arg`]: ./struct.Arg.html
    pub fn alias<S: Into<&'help str>>(mut self, name: S) -> Self {
        self.key.hidden_long(s.into());
        self
    }

    /// Allows adding [`Arg`] aliases, which function as "hidden" arguments that
    /// automatically dispatch as if this argument was used. This is more efficient, and easier
    /// than creating multiple hidden subcommands as one only needs to check for the existence of
    /// this command, and not all variants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///             .arg(Arg::new("test")
    ///                     .long("test")
    ///                     .aliases(&["do-stuff", "do-tests", "tests"])
    ///                     .help("the file to add")
    ///                     .required(false))
    ///             .get_matches_from(vec![
    ///                 "prog", "--do-tests"
    ///             ]);
    /// assert!(m.is_present("test"));
    /// ```
    /// [`Arg`]: ./struct.Arg.html
    pub fn aliases(mut self, names: &[&'help str]) -> Self {
        for &n in names {
            self.key.hidden_long(n);
        }
        self
    }

    /// Allows adding a [`Arg`] alias that functions exactly like those defined with
    /// [`Arg::alias`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///             .arg(Arg::new("test")
    ///                 .visible_alias("something-awesome")
    ///                 .long("test")
    ///                 .takes_value(true))
    ///        .get_matches_from(vec![
    ///             "prog", "--something-awesome", "coffee"
    ///         ]);
    /// assert!(m.is_present("test"));
    /// assert_eq!(m.value_of("test"), Some("coffee"));
    /// ```
    /// [`Arg`]: ./struct.Arg.html
    /// [`App::alias`]: ./struct.Arg.html#method.alias
    pub fn visible_alias<S: Into<&'help str>>(mut self, name: S) -> Self {
        self.key.long(s.into());
        self
    }

    /// Allows adding multiple [`Arg`] aliases that functions exactly like those defined
    /// with [`Arg::aliases`], except that they are visible inside the help message.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///             .arg(Arg::new("test")
    ///                 .long("test")
    ///                 .visible_aliases(&["something", "awesome", "cool"]))
    ///        .get_matches_from(vec![
    ///             "prog", "--awesome"
    ///         ]);
    /// assert!(m.is_present("test"));
    /// ```
    /// [`Arg`]: ./struct.Arg.html
    /// [`App::aliases`]: ./struct.Arg.html#method.aliases
    pub fn visible_aliases(mut self, names: &[&'help str]) -> Self {
        for &n in names {
            self.key.long(n);
        }
        self
    }

    /// Sets the short help text of the argument that will be displayed to the user when they print
    /// the help information with `-h`. Typically, this is a short (one line) description of the
    /// arg.
    ///
    /// **NOTE:** If only `Arg::help` is provided, and not [`Arg::long_help`] but the user requests
    /// `--help` clap will still display the contents of `help` appropriately
    ///
    /// **NOTE:** Only `Arg::help` is used in completion script generation in order to be concise
    ///
    /// # Examples
    ///
    /// Any valid UTF-8 is allowed in the help text. The one exception is when one wishes to
    /// include a newline in the help text and have the following text be properly aligned with all
    /// the other help text.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("config")
    ///     .help("The config file used by the myprog")
    /// # ;
    /// ```
    ///
    /// Setting `help` displays a short message to the side of the argument when the user passes
    /// `-h` or `--help` (by default).
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [FLAGS]
    ///
    /// FLAGS:
    ///     --config     Some help text describing the --config arg
    /// -h, --help       Prints help information
    /// -V, --version    Prints version information
    /// ```
    /// [`Arg::long_help`]: ./struct.Arg.html#method.long_help
    pub fn help(mut self, h: &'help str) -> Self {
        self.help.short_message(h);
        self
    }

    /// Sets the long help text of the argument that will be displayed to the user when they print
    /// the help information with `--help`. Typically this a more detailed (multi-line) message
    /// that describes the arg.
    ///
    /// **NOTE:** If only `long_help` is provided, and not [`Arg::help`] but the user requests `-h`
    /// clap will still display the contents of `long_help` appropriately
    ///
    /// **NOTE:** Only [`Arg::help`] is used in completion script generation in order to be concise
    ///
    /// # Examples
    ///
    /// Any valid UTF-8 is allowed in the help text. The one exception is when one wishes to
    /// include a newline in the help text and have the following text be properly aligned with all
    /// the other help text.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("config")
    ///     .long_help(
    /// "The config file used by the myprog must be in JSON format
    /// with only valid keys and may not contain other nonsense
    /// that cannot be read by this program. Obviously I'm going on
    /// and on, so I'll stop now.")
    /// # ;
    /// ```
    ///
    /// Setting `help` displays a short message to the side of the argument when the user passes
    /// `-h` or `--help` (by default).
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .long_help(
    /// "The config file used by the myprog must be in JSON format
    /// with only valid keys and may not contain other nonsense
    /// that cannot be read by this program. Obviously I'm going on
    /// and on, so I'll stop now."))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [FLAGS]
    ///
    /// FLAGS:
    ///    --config
    ///         The config file used by the myprog must be in JSON format
    ///         with only valid keys and may not contain other nonsense
    ///         that cannot be read by this program. Obviously I'm going on
    ///         and on, so I'll stop now.
    ///
    /// -h, --help
    ///         Prints help information
    ///
    /// -V, --version
    ///         Prints version information
    /// ```
    /// [`Arg::help`]: ./struct.Arg.html#method.help
    pub fn long_help(mut self, h: &'help str) -> Self {
        self.help.long_message(h);
        self
    }

    /// Sets an arg that override this arg's required setting. (i.e. this arg will be required
    /// unless this other argument is present).
    ///
    /// **Pro Tip:** Using [`Arg::required_unless`] implies [`Arg::required`] and is therefore not
    /// mandatory to also set.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_unless("debug")
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::required_unless(name)`] requires that the argument be used at runtime
    /// *unless* `name` is present. In the following example, the required argument is *not*
    /// provided, but it's not an error because the `unless` arg has been supplied.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless("dbg")
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting [`Arg::required_unless(name)`] and *not* supplying `name` or this arg is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless("dbg")
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug"))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::required_unless`]: ./struct.Arg.html#method.required_unless
    /// [`Arg::required`]: ./struct.Arg.html#method.required
    /// [`Arg::required_unless(name)`]: ./struct.Arg.html#method.required_unless
    pub fn required_unless<T>(mut self, other: T) -> Self where T: Hash {
        let id = hash(other);
        self.validation.self_required_rule(
            Rule::new()
                .rule_modifier(RuleModifier::Unless)
                .condition(Condition::new(id)));
        self
    }

    /// Sets args that override this arg's required setting. (i.e. this arg will be required unless
    /// all these other arguments are present).
    ///
    /// **NOTE:** If you wish for this argument to only be required if *one of* these args are
    /// present see [`Arg::required_unless_one`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_unless_all(&["cfg", "dbg"])
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::required_unless_all(names)`] requires that the argument be used at runtime
    /// *unless* *all* the args in `names` are present. In the following example, the required
    /// argument is *not* provided, but it's not an error because all the `unless` args have been
    /// supplied.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_all(&["dbg", "infile"])
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug"))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .takes_value(true))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug", "-i", "file"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting [`Arg::required_unless_all(names)`] and *not* supplying *all* of `names` or this
    /// arg is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_all(&["dbg", "infile"])
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug"))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .takes_value(true))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::required_unless_one`]: ./struct.Arg.html#method.required_unless_one
    /// [`Arg::required_unless_all(names)`]: ./struct.Arg.html#method.required_unless_all
    pub fn required_unless_all<T>(mut self, others: &[T]) -> Self where T: Hash {
        self.validation.self_required_rule(
            Rule::new()
                .rule_modifier(RuleModifier::Unless)
                .conditions_modifier(ConditionsModifier::All)
                .conditions(
                    others.iter().map(|x| Condition::new(hash(x)))));
        self
    }

    /// Sets args that override this arg's [required] setting. (i.e. this arg will be required
    /// unless *at least one of* these other arguments are present).
    ///
    /// **NOTE:** If you wish for this argument to only be required if *all of* these args are
    /// present see [`Arg::required_unless_all`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_unless_all(&["cfg", "dbg"])
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::required_unless_one(names)`] requires that the argument be used at runtime
    /// *unless* *at least one of* the args in `names` are present. In the following example, the
    /// required argument is *not* provided, but it's not an error because one the `unless` args
    /// have been supplied.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_one(&["dbg", "infile"])
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug"))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .takes_value(true))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting [`Arg::required_unless_one(names)`] and *not* supplying *at least one of* `names`
    /// or this arg is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_unless_one(&["dbg", "infile"])
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("dbg")
    ///         .long("debug"))
    ///     .arg(Arg::new("infile")
    ///         .short('i')
    ///         .takes_value(true))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [required]: ./struct.Arg.html#method.required
    /// [`Arg::required_unless_one(names)`]: ./struct.Arg.html#method.required_unless_one
    /// [`Arg::required_unless_all`]: ./struct.Arg.html#method.required_unless_all
    pub fn required_unless_one<T>(mut self, others: &[T]) -> Self where T: Hash {
        self.validation.self_required_rule(
            Rule::new()
                .rule_modifier(RuleModifier::Unless)
                .conditions(
                    others.iter().map(|x| Condition::new(hash(x)))));
        self
    }

    /// Sets a conflicting argument by name. I.e. when using this argument,
    /// the following argument can't be present and vice versa.
    ///
    /// **NOTE:** Conflicting rules take precedence over being required by default. Conflict rules
    /// only need to be set for one of the two arguments, they do not need to be set for each.
    ///
    /// **NOTE:** Defining a conflict is two-way, but does *not* need to defined for both arguments
    /// (i.e. if A conflicts with B, defining A.conflicts_with(B) is sufficient. You do not need
    /// need to also do B.conflicts_with(A))
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .conflicts_with("debug")
    /// # ;
    /// ```
    ///
    /// Setting conflicting argument, and having both arguments present at runtime is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .conflicts_with("debug")
    ///         .long("config"))
    ///     .arg(Arg::new("debug")
    ///         .long("debug"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--debug", "--config", "file.conf"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::ArgumentConflict);
    /// ```
    pub fn conflicts_with<T>(mut self, other: T) -> Self where T: Hash {
        let id = hash(other);
        self.validation.conflicts_rule(
            Rule::new()
                .condition(Condition::new(id)));
        self
    }

    /// The same as [`Arg::conflicts_with`] but allows specifying multiple two-way conlicts per
    /// argument.
    ///
    /// **NOTE:** Conflicting rules take precedence over being required by default. Conflict rules
    /// only need to be set for one of the two arguments, they do not need to be set for each.
    ///
    /// **NOTE:** Defining a conflict is two-way, but does *not* need to defined for both arguments
    /// (i.e. if A conflicts with B, defining A.conflicts_with(B) is sufficient. You do not need
    /// need to also do B.conflicts_with(A))
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .conflicts_with_all(&["debug", "input"])
    /// # ;
    /// ```
    ///
    /// Setting conflicting argument, and having any of the arguments present at runtime with a
    /// conflicting argument is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .conflicts_with_all(&["debug", "input"])
    ///         .long("config"))
    ///     .arg(Arg::new("debug")
    ///         .long("debug"))
    ///     .arg(Arg::new("input")
    ///         .index(1))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf", "file.txt"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::ArgumentConflict);
    /// ```
    /// [`Arg::conflicts_with`]: ./struct.Arg.html#method.conflicts_with
    pub fn conflicts_with_all<T>(mut self, others: &[T]) -> Self where T: Hash {
        self.validation.conflicts_rule(
            Rule::new()
                .conditions_modifier(ConditionsModifier::All)
                .conditions(otheres.iter().map(|x| Condition::new(hash(x)))));
        self
    }

    /// Sets a overridable argument by name. I.e. this argument and the following argument
    /// will override each other in POSIX style (whichever argument was specified at runtime
    /// **last** "wins")
    ///
    /// **NOTE:** When an argument is overridden it is essentially as if it never was used, any
    /// conflicts, requirements, etc. are evaluated **after** all "overrides" have been removed
    ///
    /// **WARNING:** Positional arguments and options which accept [`Multiple*`] cannot override
    /// themselves (or we would never be able to advance to the next positional). If a positional
    /// argument or option with one of the [`Multiple*`] settings lists itself as an override, it is
    /// simply ignored.
    ///
    /// # Examples
    ///
    /// ```rust # use clap::{App, Arg};
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::from("-f, --flag 'some flag'")
    ///         .conflicts_with("debug"))
    ///     .arg(Arg::from("-d, --debug 'other flag'"))
    ///     .arg(Arg::from("-c, --color 'third flag'")
    ///         .overrides_with("flag"))
    ///     .get_matches_from(vec![
    ///         "prog", "-f", "-d", "-c"]);
    ///             //    ^~~~~~~~~~~~^~~~~ flag is overridden by color
    ///
    /// assert!(m.is_present("color"));
    /// assert!(m.is_present("debug")); // even though flag conflicts with debug, it's as if flag
    ///                                 // was never used because it was overridden with color
    /// assert!(!m.is_present("flag"));
    /// ```
    /// Care must be taken when using this setting, and having an arg override with itself. This
    /// is common practice when supporting things like shell aliases, config files, etc.
    /// However, when combined with multiple values, it can get dicy.
    /// Here is how clap handles such situations:
    ///
    /// When a flag overrides itself, it's as if the flag was only ever used once (essentially
    /// preventing a "Unexpected multiple usage" error):
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("posix")
    ///             .arg(Arg::from("--flag  'some flag'").overrides_with("flag"))
    ///             .get_matches_from(vec!["posix", "--flag", "--flag"]);
    /// assert!(m.is_present("flag"));
    /// assert_eq!(m.occurrences_of("flag"), 1);
    /// ```
    /// Making a arg [`Multiple*``] and override itself is essentially meaningless. Therefore
    /// clap ignores an override of self if it's a flag and it already accepts multiple occurrences.
    ///
    /// ```
    /// # use clap::{App, Arg};
    /// let m = App::new("posix")
    ///             .arg(Arg::from("--flag...  'some flag'").overrides_with("flag"))
    ///             .get_matches_from(vec!["", "--flag", "--flag", "--flag", "--flag"]);
    /// assert!(m.is_present("flag"));
    /// assert_eq!(m.occurrences_of("flag"), 4);
    /// ```
    /// Now notice with options (which *do not* set one of the [`Multiple*`]), it's as if only the
    /// last occurrence happened.
    ///
    /// ```
    /// # use clap::{App, Arg};
    /// let m = App::new("posix")
    ///             .arg(Arg::from("--opt [val] 'some option'").overrides_with("opt"))
    ///             .get_matches_from(vec!["", "--opt=some", "--opt=other"]);
    /// assert!(m.is_present("opt"));
    /// assert_eq!(m.occurrences_of("opt"), 1);
    /// assert_eq!(m.value_of("opt"), Some("other"));
    /// ```
    ///
    /// Just like flags, options with one of the [`Multiple*``] set, will ignore the "override self"
    /// setting.
    ///
    /// ```
    /// # use clap::{App, Arg};
    /// let m = App::new("posix")
    ///             .arg(Arg::from("--opt [val]... 'some option'")
    ///                 .overrides_with("opt"))
    ///             .get_matches_from(vec!["", "--opt", "first", "over", "--opt", "other", "val"]);
    /// assert!(m.is_present("opt"));
    /// assert_eq!(m.occurrences_of("opt"), 2);
    /// assert_eq!(m.values_of("opt").unwrap().collect::<Vec<_>>(), &["first", "over", "other", "val"]);
    /// ```
    ///
    /// A safe thing to do if you'd like to support an option which supports multiple values, but
    /// also is "overridable" by itself, is to not use [`UseValueDelimiter`] and *not* use
    /// `MultipleValues` while telling users to separate values with a comma (i.e. `val1,val2`)
    ///
    /// ```
    /// # use clap::{App, Arg};
    /// let m = App::new("posix")
    ///             .arg(Arg::from("--opt [val] 'some option'")
    ///                 .overrides_with("opt"))
    ///             .get_matches_from(vec!["", "--opt=some,other", "--opt=one,two"]);
    /// assert!(m.is_present("opt"));
    /// assert_eq!(m.occurrences_of("opt"), 1);
    /// assert_eq!(m.values_of("opt").unwrap().collect::<Vec<_>>(), &["one,two"]);
    /// ```
    /// [`Multiple*`]: ./enum.ArgSettings.html#variant.MultipleValues
    /// [`UseValueDelimiter`]: ./enum.ArgSettings.html#variant.UseValueDelimiter
    pub fn overrides_with<T>(mut self, other: T) -> Self where T: Hash {
        self.validation.overrides_rule(
            Rule::new()
                .condition(Condition::new(hash(other))));
        self
    }

    /// Sets multiple mutually overridable arguments by name. I.e. this argument and the following
    /// argument will override each other in POSIX style (whichever argument was specified at
    /// runtime **last** "wins")
    ///
    /// **NOTE:** When an argument is overridden it is essentially as if it never was used, any
    /// conflicts, requirements, etc. are evaluated **after** all "overrides" have been removed
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::from("-f, --flag 'some flag'")
    ///         .conflicts_with("color"))
    ///     .arg(Arg::from("-d, --debug 'other flag'"))
    ///     .arg(Arg::from("-c, --color 'third flag'")
    ///         .overrides_with_all(&["flag", "debug"]))
    ///     .get_matches_from(vec![
    ///         "prog", "-f", "-d", "-c"]);
    ///             //    ^~~~~~^~~~~~~~~ flag and debug are overridden by color
    ///
    /// assert!(m.is_present("color")); // even though flag conflicts with color, it's as if flag
    ///                                 // and debug were never used because they were overridden
    ///                                 // with color
    /// assert!(!m.is_present("debug"));
    /// assert!(!m.is_present("flag"));
    /// ```
    pub fn overrides_with_all<T>(mut self, others: &[T]) -> Self where T: Hash {
        self.validation.overrides_rule(
            Rule::new()
                .conditions_modifier(ConditionsModifier::All)
                .conditions(others.iter().map(|x| Condition::new(hash(x)))));
        self
    }

    /// Sets an argument by name that is required when this one is present I.e. when
    /// using this argument, the following argument *must* be present.
    ///
    /// **NOTE:** [Conflicting] rules and [override] rules take precedence over being required
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires("input")
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::requires(name)`] requires that the argument be used at runtime if the
    /// defining argument is used. If the defining argument isn't used, the other argument isn't
    /// required
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .requires("input")
    ///         .long("config"))
    ///     .arg(Arg::new("input")
    ///         .index(1))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use cfg, so input wasn't required
    /// ```
    ///
    /// Setting [`Arg::requires(name)`] and *not* supplying that argument is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .requires("input")
    ///         .long("config"))
    ///     .arg(Arg::new("input")
    ///         .index(1))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: ./struct.Arg.html#method.requires
    /// [Conflicting]: ./struct.Arg.html#method.conflicts_with
    /// [override]: ./struct.Arg.html#method.overrides_with
    pub fn requires<T>(mut self, other: T) -> Self where T: Hash {
        self.validation.requirements_rule(
            Rule::new()
                .condition(Condition::new(hash(other))));
        self
    }

    /// Allows a conditional requirement. The requirement will only become valid if this arg's value
    /// equals `val`.
    ///
    /// **NOTE:** If using YAML the values should be laid out as follows
    ///
    /// ```yaml
    /// requires_if:
    ///     - [val, arg]
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires_if("val", "arg")
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::requires_if(val, arg)`] requires that the `arg` be used at runtime if the
    /// defining argument's value is equal to `val`. If the defining argument is anything other than
    /// `val`, the other argument isn't required.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .requires_if("my.cfg", "other")
    ///         .long("config"))
    ///     .arg(Arg::new("other"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "some.cfg"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use --config=my.cfg, so other wasn't required
    /// ```
    ///
    /// Setting [`Arg::requires_if(val, arg)`] and setting the value to `val` but *not* supplying
    /// `arg` is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .requires_if("my.cfg", "input")
    ///         .long("config"))
    ///     .arg(Arg::new("input"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "my.cfg"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: ./struct.Arg.html#method.requires
    /// [Conflicting]: ./struct.Arg.html#method.conflicts_with
    /// [override]: ./struct.Arg.html#method.overrides_with
    pub fn requires_if<T>(mut self, val: &'help str, other: T) -> Self where T: Hash {
        // need self val and other val...have to re-think
        self.validation.requirements_rule(
            Rule::new()
                .condition(Condition::new(hash(other))));
        self
    }

    /// Allows multiple conditional requirements. The requirement will only become valid if this arg's value
    /// equals `val`.
    ///
    /// **NOTE:** If using YAML the values should be laid out as follows
    ///
    /// ```yaml
    /// requires_if:
    ///     - [val, arg]
    ///     - [val2, arg2]
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires_ifs(&[
    ///         ("val", "arg"),
    ///         ("other_val", "arg2"),
    ///     ])
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::requires_ifs(&["val", "arg"])`] requires that the `arg` be used at runtime if the
    /// defining argument's value is equal to `val`. If the defining argument's value is anything other
    /// than `val`, `arg` isn't required.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .requires_ifs(&[
    ///             ("special.conf", "opt"),
    ///             ("other.conf", "other"),
    ///         ])
    ///         .long("config"))
    ///     .arg(Arg::new("opt")
    ///         .long("option")
    ///         .takes_value(true))
    ///     .arg(Arg::new("other"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "special.conf"
    ///     ]);
    ///
    /// assert!(res.is_err()); // We  used --config=special.conf so --option <val> is required
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: ./struct.Arg.html#method.requires
    /// [Conflicting]: ./struct.Arg.html#method.conflicts_with
    /// [override]: ./struct.Arg.html#method.overrides_with
    pub fn requires_ifs<T>(mut self, ifs: &[(&'help str, T)]) -> Self where T: Hash {
        if let Some(ref mut vec) = self.requires {
            for &(val, other) in ifs {
                vec.push((Some(val), hash(other)));
            }
        } else {
            let mut vec = vec![];
            for &(val, other) in ifs {
                vec.push((Some(val), hash(other)));
            }
            self.requires = Some(vec);
        }
        self
    }

    /// Allows specifying that an argument is [required] conditionally. The requirement will only
    /// become valid if the specified `arg`'s value equals `val`.
    ///
    /// **NOTE:** If using YAML the values should be laid out as follows
    ///
    /// ```yaml
    /// required_if:
    ///     - [arg, val]
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_if("other_arg", "value")
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::required_if(arg, val)`] makes this arg required if the `arg` is used at
    /// runtime and it's value is equal to `val`. If the `arg`'s value is anything other than `val`,
    /// this argument isn't required.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .required_if("other", "special")
    ///         .long("config"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .takes_value(true))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--other", "not-special"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use --other=special, so "cfg" wasn't required
    /// ```
    ///
    /// Setting [`Arg::required_if(arg, val)`] and having `arg` used with a value of `val` but *not*
    /// using this arg is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .required_if("other", "special")
    ///         .long("config"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .takes_value(true))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--other", "special"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: ./struct.Arg.html#method.requires
    /// [Conflicting]: ./struct.Arg.html#method.conflicts_with
    /// [required]: ./struct.Arg.html#method.required
    pub fn required_if<T>(mut self, other: T, val: &'help str) -> Self where T: Hash {
        let id = hash(other);
        if let Some(ref mut vec) = self.r_ifs {
            vec.push((id, val));
        } else {
            self.r_ifs = Some(vec![(id, val)]);
        }
        self
    }

    /// Allows specifying that an argument is [required] based on multiple conditions. The
    /// conditions are set up in a `(arg, val)` style tuple. The requirement will only become valid
    /// if one of the specified `arg`'s value equals it's corresponding `val`.
    ///
    /// **NOTE:** If using YAML the values should be laid out as follows
    ///
    /// ```yaml
    /// required_if:
    ///     - [arg, val]
    ///     - [arg2, val2]
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .required_ifs(&[
    ///         ("extra", "val"),
    ///         ("option", "spec")
    ///     ])
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::required_ifs(&[(arg, val)])`] makes this arg required if any of the `arg`s
    /// are used at runtime and it's corresponding value is equal to `val`. If the `arg`'s value is
    /// anything other than `val`, this argument isn't required.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_ifs(&[
    ///             ("extra", "val"),
    ///             ("option", "spec")
    ///         ])
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("extra")
    ///         .takes_value(true)
    ///         .long("extra"))
    ///     .arg(Arg::new("option")
    ///         .takes_value(true)
    ///         .long("option"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--option", "other"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use --option=spec, or --extra=val so "cfg" isn't required
    /// ```
    ///
    /// Setting [`Arg::required_ifs(&[(arg, val)])`] and having any of the `arg`s used with it's
    /// value of `val` but *not* using this arg is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .required_ifs(&[
    ///             ("extra", "val"),
    ///             ("option", "spec")
    ///         ])
    ///         .takes_value(true)
    ///         .long("config"))
    ///     .arg(Arg::new("extra")
    ///         .takes_value(true)
    ///         .long("extra"))
    ///     .arg(Arg::new("option")
    ///         .takes_value(true)
    ///         .long("option"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--option", "spec"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Arg::requires(name)`]: ./struct.Arg.html#method.requires
    /// [Conflicting]: ./struct.Arg.html#method.conflicts_with
    /// [required]: ./struct.Arg.html#method.required
    pub fn required_ifs<T>(mut self, ifs: &[(T, &'help str)]) -> Self where T: Hash {
        if let Some(ref mut vec) = self.r_ifs {
            for r_if in ifs {
                vec.push((hash(r_if.0), r_if.1));
            }
        } else {
            let mut vec = vec![];
            for r_if in ifs {
                vec.push((hash(r_if.0), r_if.1));
            }
            self.r_ifs = Some(vec);
        }
        self
    }

    /// Sets multiple arguments by names that are required when this one is present I.e. when
    /// using this argument, the following arguments *must* be present.
    ///
    /// **NOTE:** [Conflicting] rules and [override] rules take precedence over being required
    /// by default.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::Arg;
    /// Arg::new("config")
    ///     .requires_all(&["input", "output"])
    /// # ;
    /// ```
    ///
    /// Setting [`Arg::requires_all(&[arg, arg2])`] requires that all the arguments be used at
    /// runtime if the defining argument is used. If the defining argument isn't used, the other
    /// argument isn't required
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .requires("input")
    ///         .long("config"))
    ///     .arg(Arg::new("input")
    ///         .index(1))
    ///     .arg(Arg::new("output")
    ///         .index(2))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_ok()); // We didn't use cfg, so input and output weren't required
    /// ```
    ///
    /// Setting [`Arg::requires_all(&[arg, arg2])`] and *not* supplying all the arguments is an
    /// error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .takes_value(true)
    ///         .requires_all(&["input", "output"])
    ///         .long("config"))
    ///     .arg(Arg::new("input")
    ///         .index(1))
    ///     .arg(Arg::new("output")
    ///         .index(2))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf", "in.txt"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// // We didn't use output
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [Conflicting]: ./struct.Arg.html#method.conflicts_with
    /// [override]: ./struct.Arg.html#method.overrides_with
    /// [`Arg::requires_all(&[arg, arg2])`]: ./struct.Arg.html#method.requires_all
    pub fn requires_all<T>(mut self, others: &[T]) -> Self where T: Hash {
        if let Some(ref mut vec) = self.requires {
            for s in others {
                vec.push((None, hash(s)));
            }
        } else {
            let mut vec = vec![];
            for s in others {
                vec.push((None, hash(s)));
            }
            self.requires = Some(vec);
        }
        self
    }

    /// Specifies the index of a positional argument **starting at** 1.
    ///
    /// **NOTE:** The index refers to position according to **other positional argument**. It does
    /// not define position in the argument list as a whole.
    ///
    /// **NOTE:** If no [`Arg::short`], or [`Arg::long`] have been defined, you can optionally
    /// leave off the `index` method, and the index will be assigned in order of evaluation.
    /// Utilizing the `index` method allows for setting indexes out of order
    ///
    /// **NOTE:** When utilized with [`Arg::multiple(true)`], only the **last** positional argument
    /// may be defined as multiple (i.e. with the highest index)
    ///
    /// # Panics
    ///
    /// Although not in this method directly, [`App`] will [`panic!`] if indexes are skipped (such
    /// as defining `index(1)` and `index(3)` but not `index(2)`, or a positional argument is
    /// defined as multiple and is not the highest index
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("config")
    ///     .index(1)
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .index(1))
    ///     .arg(Arg::new("debug")
    ///         .long("debug"))
    ///     .get_matches_from(vec![
    ///         "prog", "--debug", "fast"
    ///     ]);
    ///
    /// assert!(m.is_present("mode"));
    /// assert_eq!(m.value_of("mode"), Some("fast")); // notice index(1) means "first positional"
    ///                                               // *not* first argument
    /// ```
    /// [`Arg::short`]: ./struct.Arg.html#method.short
    /// [`Arg::long`]: ./struct.Arg.html#method.long
    /// [`Arg::multiple(true)`]: ./struct.Arg.html#method.multiple
    /// [`App`]: ./struct.App.html
    /// [`panic!`]: https://doc.rust-lang.org/std/macro.panic!.html
    pub fn index(mut self, idx: u64) -> Self {
        self.key.index(idx);
        self
    }

    /// Specifies a value that *stops* parsing multiple values of a give argument. By default when
    /// one sets [`multiple(true)`] on an argument, clap will continue parsing values for that
    /// argument until it reaches another valid argument, or one of the other more specific settings
    /// for multiple values is used (such as [`min_values`], [`max_values`] or
    /// [`number_of_values`]).
    ///
    /// **NOTE:** This setting only applies to [options] and [positional arguments]
    ///
    /// **NOTE:** When the terminator is passed in on the command line, it is **not** stored as one
    /// of the values
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("vals")
    ///     .takes_value(true)
    ///     .multiple(true)
    ///     .value_terminator(";")
    /// # ;
    /// ```
    /// The following example uses two arguments, a sequence of commands, and the location in which
    /// to perform them
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cmds")
    ///         .multiple(true)
    ///         .allow_hyphen_values(true)
    ///         .value_terminator(";"))
    ///     .arg(Arg::new("location"))
    ///     .get_matches_from(vec![
    ///         "prog", "find", "-type", "f", "-name", "special", ";", "/home/clap"
    ///     ]);
    /// let cmds: Vec<_> = m.values_of("cmds").unwrap().collect();
    /// assert_eq!(&cmds, &["find", "-type", "f", "-name", "special"]);
    /// assert_eq!(m.value_of("location"), Some("/home/clap"));
    /// ```
    /// [options]: ./struct.Arg.html#method.takes_value
    /// [positional arguments]: ./struct.Arg.html#method.index
    /// [`multiple(true)`]: ./struct.Arg.html#method.multiple
    /// [`min_values`]: ./struct.Arg.html#method.min_values
    /// [`number_of_values`]: ./struct.Arg.html#method.number_of_values
    /// [`max_values`]: ./struct.Arg.html#method.max_values
    pub fn value_terminator(mut self, term: &'help str) -> Self {
        self.value.terminator(term);
        self
    }

    /// Specifies a list of possible values for this argument. At runtime, `clap` verifies that
    /// only one of the specified values was used, or fails with an error message.
    ///
    /// **NOTE:** This setting only applies to [options] and [positional arguments]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("mode")
    ///     .takes_value(true)
    ///     .possible_values(&["fast", "slow", "medium"])
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .takes_value(true)
    ///         .possible_values(&["fast", "slow", "medium"]))
    ///     .get_matches_from(vec![
    ///         "prog", "--mode", "fast"
    ///     ]);
    /// assert!(m.is_present("mode"));
    /// assert_eq!(m.value_of("mode"), Some("fast"));
    /// ```
    ///
    /// The next example shows a failed parse from using a value which wasn't defined as one of the
    /// possible values.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .takes_value(true)
    ///         .possible_values(&["fast", "slow", "medium"]))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--mode", "wrong"
    ///     ]);
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::InvalidValue);
    /// ```
    /// [options]: ./struct.Arg.html#method.takes_value
    /// [positional arguments]: ./struct.Arg.html#method.index
    pub fn possible_values(mut self, values: &[&'help str]) -> Self {
        if let Some(ref mut vec) = self.possible_vals {
            for s in values {
                vec.push(s);
            }
        } else {
            self.possible_vals = Some(values.to_vec());
        }
        self
    }

    /// Specifies a possible value for this argument, one at a time. At runtime, `clap` verifies
    /// that only one of the specified values was used, or fails with error message.
    ///
    /// **NOTE:** This setting only applies to [options] and [positional arguments]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("mode")
    ///     .takes_value(true)
    ///     .possible_value("fast")
    ///     .possible_value("slow")
    ///     .possible_value("medium")
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .takes_value(true)
    ///         .possible_value("fast")
    ///         .possible_value("slow")
    ///         .possible_value("medium"))
    ///     .get_matches_from(vec![
    ///         "prog", "--mode", "fast"
    ///     ]);
    /// assert!(m.is_present("mode"));
    /// assert_eq!(m.value_of("mode"), Some("fast"));
    /// ```
    ///
    /// The next example shows a failed parse from using a value which wasn't defined as one of the
    /// possible values.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .takes_value(true)
    ///         .possible_value("fast")
    ///         .possible_value("slow")
    ///         .possible_value("medium"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--mode", "wrong"
    ///     ]);
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::InvalidValue);
    /// ```
    /// [options]: ./struct.Arg.html#method.takes_value
    /// [positional arguments]: ./struct.Arg.html#method.index
    pub fn possible_value(mut self, value: &'help str) -> Self {
        if let Some(ref mut vec) = self.possible_vals {
            vec.push(value);
        } else {
            self.possible_vals = Some(vec![value]);
        }
        self
    }

    /// Specifies how many values are required to satisfy this argument. For example, if you had a
    /// `-f <file>` argument where you wanted exactly 3 'files' you would set
    /// `.number_of_values(3)`, and this argument wouldn't be satisfied unless the user provided
    /// 3 and only 3 values.
    ///
    /// **NOTE:** Does *not* require [`Arg::multiple(true)`] to be set. Setting
    /// [`Arg::multiple(true)`] would allow `-f <file> <file> <file> -f <file> <file> <file>` where
    /// as *not* setting [`Arg::multiple(true)`] would only allow one occurrence of this argument.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("file")
    ///     .short('f')
    ///     .number_of_values(3)
    /// # ;
    /// ```
    ///
    /// Not supplying the correct number of values is an error
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .takes_value(true)
    ///         .number_of_values(2)
    ///         .short('F'))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::WrongNumberOfValues);
    /// ```
    /// [`Arg::multiple(true)`]: ./struct.Arg.html#method.multiple
    pub fn number_of_values(mut self, qty: u64) -> Self {
        self.num_vals = Some(qty);
        self
    }

    /// Specifies how many values are required to be present pre occurrence of this argument.
    ///
    /// **NOTE:** Does *not* require [`Arg::multiple(true)`] to be set. Setting
    /// [`Arg::multiple(true)`] would allow `-f <file> <file> <file> -f <file> <file> <file>` where
    /// as *not* setting [`Arg::multiple(true)`] would only allow one occurrence of this argument.
    ///
    /// **NOTE:** Implies `Arg::multiple_occurrences(true)`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("file")
    ///     .short('f')
    ///     .number_of_values(3)
    /// # ;
    /// ```
    ///
    /// Not supplying the correct number of values is an error
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .takes_value(true)
    ///         .number_of_values_per_occurrence(2)
    ///         .short('F'))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::WrongNumberOfValues);
    /// ```
    /// [`Arg::multiple(true)`]: ./struct.Arg.html#method.multiple
    pub fn number_of_values_per_occurrence(mut self, qty: u64) -> Self {
        self.num_vals_per_occ = Some(qty);
        self
    }

    /// Allows one to perform a custom validation on the argument value. You provide a closure
    /// which accepts a [`String`] value, and return a [`Result`] where the [`Err(String)`] is a
    /// message displayed to the user.
    ///
    /// **NOTE:** The error message does *not* need to contain the `error:` portion, only the
    /// message as all errors will appear as
    /// `error: Invalid value for '<arg>': <YOUR MESSAGE>` where `<arg>` is replaced by the actual
    /// arg, and `<YOUR MESSAGE>` is the `String` you return as the error.
    ///
    /// **NOTE:** There is a small performance hit for using validators, as they are implemented
    /// with [`Rc`] pointers. And the value to be checked will be allocated an extra time in order
    /// to to be passed to the closure. This performance hit is extremely minimal in the grand
    /// scheme of things.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// fn has_at(v: String) -> Result<(), String> {
    ///     if v.contains("@") { return Ok(()); }
    ///     Err(String::from("The value did not contain the required @ sigil"))
    /// }
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .index(1)
    ///         .validator(has_at))
    ///     .try_get_matches_from(vec![
    ///         "prog", "some@file"
    ///     ]);
    /// assert!(res.is_ok());
    /// assert_eq!(res.unwrap().value_of("file"), Some("some@file"));
    /// ```
    /// [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
    /// [`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html
    /// [`Err(String)`]: https://doc.rust-lang.org/std/result/enum.Result.html#variant.Err
    /// [`Rc`]: https://doc.rust-lang.org/std/rc/struct.Rc.html
    pub fn validator<F, O, E>(mut self, f: F) -> Self
        where
            F: Fn(String) -> Result<O, E> + 'static,
            E: ToString,
    {
        self.validator = Some(Rc::new(move |s| {
            f(s).map(|_| ()).map_err(|e| e.to_string())
        }));
        self
    }

    /// Works identically to Validator but is intended to be used with values that could
    /// contain non UTF-8 formatted strings.
    ///
    /// # Examples
    ///
    #[cfg_attr(not(unix), doc = " ```ignore")]
    #[cfg_attr(unix, doc = " ```rust")]
    /// # use clap::{App, Arg};
    /// # use std::ffi::{OsStr, OsString};
    /// # use std::os::unix::ffi::OsStrExt;
    /// fn has_ampersand(v: &OsStr) -> Result<(), String> {
    ///     if v.as_bytes().iter().any(|b| *b == b'&') { return Ok(()); }
    ///     Err(String::from("The value did not contain the required & sigil"))
    /// }
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .index(1)
    ///         .validator_os(has_ampersand))
    ///     .try_get_matches_from(vec![
    ///         "prog", "Fish & chips"
    ///     ]);
    /// assert!(res.is_ok());
    /// assert_eq!(res.unwrap().value_of("file"), Some("Fish & chips"));
    /// ```
    /// [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
    /// [`OsStr`]: https://doc.rust-lang.org/std/ffi/struct.OsStr.html
    /// [`OsString`]: https://doc.rust-lang.org/std/ffi/struct.OsString.html
    /// [`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html
    /// [`Err(String)`]: https://doc.rust-lang.org/std/result/enum.Result.html#variant.Err
    /// [`Rc`]: https://doc.rust-lang.org/std/rc/struct.Rc.html
    pub fn validator_os<F, O>(mut self, f: F) -> Self
        where
            F: Fn(&OsStr) -> Result<O, String> + 'static,
    {
        self.validator_os = Some(Rc::new(move |s| f(s).map(|_| ())));
        self
    }

    /// Specifies the *maximum* number of values are for this argument. For example, if you had a
    /// `-f <file>` argument where you wanted up to 3 'files' you would set `.max_values(3)`, and
    /// this argument would be satisfied if the user provided, 1, 2, or 3 values.
    ///
    /// **NOTE:** This does *not* implicitly set [`Arg::multiple(true)`]. This is because
    /// `-o val -o val` is multiple occurrences but a single value and `-o val1 val2` is a single
    /// occurrence with multiple values. For positional arguments this **does** set
    /// [`Arg::multiple(true)`] because there is no way to determine the difference between multiple
    /// occurrences and multiple values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("file")
    ///     .short('f')
    ///     .max_values(3)
    /// # ;
    /// ```
    ///
    /// Supplying less than the maximum number of values is allowed
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .takes_value(true)
    ///         .max_values(3)
    ///         .short('F'))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1", "file2"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// let m = res.unwrap();
    /// let files: Vec<_> = m.values_of("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2"]);
    /// ```
    ///
    /// Supplying more than the maximum number of values is an error
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .takes_value(true)
    ///         .max_values(2)
    ///         .short('F'))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1", "file2", "file3"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::TooManyValues);
    /// ```
    /// [`Arg::multiple(true)`]: ./struct.Arg.html#method.multiple
    pub fn max_values(mut self, qty: u64) -> Self {
        self.max_vals = Some(qty);
        self
    }

    /// Specifies the *minimum* number of values for this argument. For example, if you had a
    /// `-f <file>` argument where you wanted at least 2 'files' you would set
    /// `.min_values(2)`, and this argument would be satisfied if the user provided, 2 or more
    /// values.
    ///
    /// **NOTE:** This does not implicitly set [`Arg::multiple(true)`]. This is because
    /// `-o val -o val` is multiple occurrences but a single value and `-o val1 val2` is a single
    /// occurrence with multiple values. For positional arguments this **does** set
    /// [`Arg::multiple(true)`] because there is no way to determine the difference between multiple
    /// occurrences and multiple values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("file")
    ///     .short('f')
    ///     .min_values(3)
    /// # ;
    /// ```
    ///
    /// Supplying more than the minimum number of values is allowed
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .takes_value(true)
    ///         .min_values(2)
    ///         .short('F'))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1", "file2", "file3"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// let m = res.unwrap();
    /// let files: Vec<_> = m.values_of("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3"]);
    /// ```
    ///
    /// Supplying less than the minimum number of values is an error
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .takes_value(true)
    ///         .min_values(2)
    ///         .short('F'))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::TooFewValues);
    /// ```
    /// [`Arg::multiple(true)`]: ./struct.Arg.html#method.multiple
    pub fn min_values(mut self, qty: u64) -> Self {
        self.min_vals = Some(qty);
        self
    }

    /// Specifies the separator to use when values are clumped together, defaults to `,` (comma).
    ///
    /// **NOTE:** implicitly sets [`Arg::use_delimiter(true)`]
    ///
    /// **NOTE:** implicitly sets [`Arg::takes_value(true)`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("config")
    ///         .short('c')
    ///         .long("config")
    ///         .value_delimiter(";"))
    ///     .get_matches_from(vec![
    ///         "prog", "--config=val1;val2;val3"
    ///     ]);
    ///
    /// assert_eq!(m.values_of("config").unwrap().collect::<Vec<_>>(), ["val1", "val2", "val3"])
    /// ```
    /// [`Arg::use_delimiter(true)`]: ./struct.Arg.html#method.use_delimiter
    /// [`Arg::takes_value(true)`]: ./struct.Arg.html#method.takes_value
    pub fn value_delimiter(mut self, d: &str) -> Self {
        self.val_delim = Some(
            d.chars()
                .nth(0)
                .expect("Failed to get value_delimiter from arg"),
        );
        self
    }

    /// Specify multiple names for values of option arguments. These names are cosmetic only, used
    /// for help and usage strings only. The names are **not** used to access arguments. The values
    /// of the arguments are accessed in numeric order (i.e. if you specify two names `one` and
    /// `two` `one` will be the first matched value, `two` will be the second).
    ///
    /// This setting can be very helpful when describing the type of input the user should be
    /// using, such as `FILE`, `INTERFACE`, etc. Although not required, it's somewhat convention to
    /// use all capital letters for the value name.
    ///
    /// **Pro Tip:** It may help to use [`Arg::next_line_help(true)`] if there are long, or
    /// multiple value names in order to not throw off the help text alignment of all options.
    ///
    /// **NOTE:** This implicitly sets [`Arg::number_of_values`] if the number of value names is
    /// greater than one. I.e. be aware that the number of "names" you set for the values, will be
    /// the *exact* number of values required to satisfy this argument
    ///
    /// **NOTE:** implicitly sets [`Arg::takes_value(true)`]
    ///
    /// **NOTE:** Does *not* require or imply [`Arg::multiple(true)`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("speed")
    ///     .short('s')
    ///     .value_names(&["fast", "slow"])
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("io")
    ///         .long("io-files")
    ///         .value_names(&["INFILE", "OUTFILE"]))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    /// Running the above program produces the following output
    ///
    /// ```notrust
    /// valnames
    ///
    /// USAGE:
    ///    valnames [FLAGS] [OPTIONS]
    ///
    /// FLAGS:
    ///     -h, --help       Prints help information
    ///     -V, --version    Prints version information
    ///
    /// OPTIONS:
    ///     --io-files <INFILE> <OUTFILE>    Some help text
    /// ```
    /// [`Arg::next_line_help(true)`]: ./struct.Arg.html#method.next_line_help
    /// [`Arg::number_of_values`]: ./struct.Arg.html#method.number_of_values
    /// [`Arg::takes_value(true)`]: ./struct.Arg.html#method.takes_value
    /// [`Arg::multiple(true)`]: ./struct.Arg.html#method.multiple
    pub fn value_names(mut self, names: &[&'help str]) -> Self {
        if let Some(ref mut vals) = self.val_names {
            let mut l = vals.len();
            for s in names {
                vals.insert(l, s);
                l += 1;
            }
        } else {
            let mut vm = VecMap::new();
            for (i, n) in names.iter().enumerate() {
                vm.insert(i, *n);
            }
            self.val_names = Some(vm);
        }
        self
    }

    /// Specifies the name for value of [option] or [positional] arguments inside of help
    /// documentation. This name is cosmetic only, the name is **not** used to access arguments.
    /// This setting can be very helpful when describing the type of input the user should be
    /// using, such as `FILE`, `INTERFACE`, etc. Although not required, it's somewhat convention to
    /// use all capital letters for the value name.
    ///
    /// **NOTE:** implicitly sets [`Arg::takes_value(true)`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("cfg")
    ///     .long("config")
    ///     .value_name("FILE")
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("config")
    ///         .long("config")
    ///         .value_name("FILE"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    /// Running the above program produces the following output
    ///
    /// ```notrust
    /// valnames
    ///
    /// USAGE:
    ///    valnames [FLAGS] [OPTIONS]
    ///
    /// FLAGS:
    ///     -h, --help       Prints help information
    ///     -V, --version    Prints version information
    ///
    /// OPTIONS:
    ///     --config <FILE>     Some help text
    /// ```
    /// [option]: ./struct.Arg.html#method.takes_value
    /// [positional]: ./struct.Arg.html#method.index
    /// [`Arg::takes_value(true)`]: ./struct.Arg.html#method.takes_value
    pub fn value_name(mut self, name: &'help str) -> Self {
        if let Some(ref mut vals) = self.val_names {
            let l = vals.len();
            vals.insert(l, name);
        } else {
            let mut vm = VecMap::new();
            vm.insert(0, name);
            self.val_names = Some(vm);
        }
        self
    }

    /// Specifies the value of the argument when *not* specified at runtime.
    ///
    /// **NOTE:** If the user *does not* use this argument at runtime, [`ArgMatches::occurrences_of`]
    /// will return `0` even though the [`ArgMatches::value_of`] will return the default specified.
    ///
    /// **NOTE:** If the user *does not* use this argument at runtime [`ArgMatches::is_present`] will
    /// still return `true`. If you wish to determine whether the argument was used at runtime or
    /// not, consider [`ArgMatches::occurrences_of`] which will return `0` if the argument was *not*
    /// used at runtime.
    ///
    /// **NOTE:** This setting is perfectly compatible with [`Arg::default_value_if`] but slightly
    /// different. `Arg::default_value` *only* takes affect when the user has not provided this arg
    /// at runtime. `Arg::default_value_if` however only takes affect when the user has not provided
    /// a value at runtime **and** these other conditions are met as well. If you have set
    /// `Arg::default_value` and `Arg::default_value_if`, and the user **did not** provide a this
    /// arg at runtime, nor did were the conditions met for `Arg::default_value_if`, the
    /// `Arg::default_value` will be applied.
    ///
    /// **NOTE:** This implicitly sets [`Arg::takes_value(true)`].
    ///
    /// **NOTE:** This setting effectively disables `AppSettings::ArgRequiredElseHelp` if used in
    /// conjunction as it ensures that some argument will always be present.
    ///
    /// # Examples
    ///
    /// First we use the default value without providing any value at runtime.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .long("myopt")
    ///         .default_value("myval"))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("opt"), Some("myval"));
    /// assert!(m.is_present("opt"));
    /// assert_eq!(m.occurrences_of("opt"), 0);
    /// ```
    ///
    /// Next we provide a value at runtime to override the default.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .long("myopt")
    ///         .default_value("myval"))
    ///     .get_matches_from(vec![
    ///         "prog", "--myopt=non_default"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("opt"), Some("non_default"));
    /// assert!(m.is_present("opt"));
    /// assert_eq!(m.occurrences_of("opt"), 1);
    /// ```
    /// [`ArgMatches::occurrences_of`]: ./struct.ArgMatches.html#method.occurrences_of
    /// [`ArgMatches::value_of`]: ./struct.ArgMatches.html#method.value_of
    /// [`Arg::takes_value(true)`]: ./struct.Arg.html#method.takes_value
    /// [`ArgMatches::is_present`]: ./struct.ArgMatches.html#method.is_present
    /// [`Arg::default_value_if`]: ./struct.Arg.html#method.default_value_if
    pub fn default_value(self, val: &'help str) -> Self {
        self.default_value_os(OsStr::from_bytes(val.as_bytes()))
    }

    /// Provides a default value in the exact same manner as [`Arg::default_value`]
    /// only using [`OsStr`]s instead.
    /// [`Arg::default_value`]: ./struct.Arg.html#method.default_value
    /// [`OsStr`]: https://doc.rust-lang.org/std/ffi/struct.OsStr.html
    pub fn default_value_os(mut self, val: &'help OsStr) -> Self {
        self.default_val = Some(val);
        self
    }

    /// Specifies the value of the argument if `arg` has been used at runtime. If `val` is set to
    /// `None`, `arg` only needs to be present. If `val` is set to `"some-val"` then `arg` must be
    /// present at runtime **and** have the value `val`.
    ///
    /// **NOTE:** This setting is perfectly compatible with [`Arg::default_value`] but slightly
    /// different. `Arg::default_value` *only* takes affect when the user has not provided this arg
    /// at runtime. This setting however only takes affect when the user has not provided a value at
    /// runtime **and** these other conditions are met as well. If you have set `Arg::default_value`
    /// and `Arg::default_value_if`, and the user **did not** provide a this arg at runtime, nor did
    /// were the conditions met for `Arg::default_value_if`, the `Arg::default_value` will be
    /// applied.
    ///
    /// **NOTE:** This implicitly sets [`Arg::takes_value(true)`].
    ///
    /// **NOTE:** If using YAML the values should be laid out as follows (`None` can be represented
    /// as `null` in YAML)
    ///
    /// ```yaml
    /// default_value_if:
    ///     - [arg, val, default]
    /// ```
    ///
    /// # Examples
    ///
    /// First we use the default value only if another arg is present at runtime.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("flag", None, "default"))
    ///     .get_matches_from(vec![
    ///         "prog", "--flag"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("other"), Some("default"));
    /// ```
    ///
    /// Next we run the same test, but without providing `--flag`.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("flag", None, "default"))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("other"), None);
    /// ```
    ///
    /// Now lets only use the default value if `--opt` contains the value `special`.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .takes_value(true)
    ///         .long("opt"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("opt", Some("special"), "default"))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "special"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("other"), Some("default"));
    /// ```
    ///
    /// We can run the same test and provide any value *other than* `special` and we won't get a
    /// default value.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .takes_value(true)
    ///         .long("opt"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_if("opt", Some("special"), "default"))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "hahaha"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("other"), None);
    /// ```
    /// [`Arg::takes_value(true)`]: ./struct.Arg.html#method.takes_value
    /// [`Arg::default_value`]: ./struct.Arg.html#method.default_value
    pub fn default_value_if<T>(self, arg: T, val: Option<&'help str>, default: &'help str) -> Self where T: Hash {
        self.default_value_if_os(
            arg,
            val.map(str::as_bytes).map(OsStr::from_bytes),
            OsStr::from_bytes(default.as_bytes()),
        )
    }

    /// Provides a conditional default value in the exact same manner as [`Arg::default_value_if`]
    /// only using [`OsStr`]s instead.
    /// [`Arg::default_value_if`]: ./struct.Arg.html#method.default_value_if
    /// [`OsStr`]: https://doc.rust-lang.org/std/ffi/struct.OsStr.html
    pub fn default_value_if_os<T>(
        mut self,
        arg: T,
        val: Option<&'help OsStr>,
        default: &'help OsStr,
    ) -> Self where T: Hash {
        let id = hash(arg);
        if let Some(ref mut vm) = self.default_vals_ifs {
            let l = vm.len();
            vm.insert(l, (id, val, default));
        } else {
            let mut vm = VecMap::new();
            vm.insert(0, (id, val, default));
            self.default_vals_ifs = Some(vm);
        }
        self
    }

    /// Specifies multiple values and conditions in the same manner as [`Arg::default_value_if`].
    /// The method takes a slice of tuples in the `(arg, Option<val>, default)` format.
    ///
    /// **NOTE**: The conditions are stored in order and evaluated in the same order. I.e. the first
    /// if multiple conditions are true, the first one found will be applied and the ultimate value.
    ///
    /// **NOTE:** If using YAML the values should be laid out as follows
    ///
    /// ```yaml
    /// default_value_if:
    ///     - [arg, val, default]
    ///     - [arg2, null, default2]
    /// ```
    ///
    /// # Examples
    ///
    /// First we use the default value only if another arg is present at runtime.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag"))
    ///     .arg(Arg::new("opt")
    ///         .long("opt")
    ///         .takes_value(true))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_ifs(&[
    ///             ("flag", None, "default"),
    ///             ("opt", Some("channal"), "chan"),
    ///         ]))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "channal"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("other"), Some("chan"));
    /// ```
    ///
    /// Next we run the same test, but without providing `--flag`.
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag"))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_ifs(&[
    ///             ("flag", None, "default"),
    ///             ("opt", Some("channal"), "chan"),
    ///         ]))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("other"), None);
    /// ```
    ///
    /// We can also see that these values are applied in order, and if more than one condition is
    /// true, only the first evaluated "wins"
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag"))
    ///     .arg(Arg::new("opt")
    ///         .long("opt")
    ///         .takes_value(true))
    ///     .arg(Arg::new("other")
    ///         .long("other")
    ///         .default_value_ifs(&[
    ///             ("flag", None, "default"),
    ///             ("opt", Some("channal"), "chan"),
    ///         ]))
    ///     .get_matches_from(vec![
    ///         "prog", "--opt", "channal", "--flag"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("other"), Some("default"));
    /// ```
    /// [`Arg::takes_value(true)`]: ./struct.Arg.html#method.takes_value
    /// [`Arg::default_value`]: ./struct.Arg.html#method.default_value
    pub fn default_value_ifs<T>(mut self, ifs: &[(T, Option<&'help str>, &'help str)]) -> Self where T: Hash {
        for &(arg, val, default) in ifs {
            self = self.default_value_if_os(
                arg,
                val.map(str::as_bytes).map(OsStr::from_bytes),
                OsStr::from_bytes(default.as_bytes()),
            );
        }
        self
    }

    /// Provides multiple conditional default values in the exact same manner as
    /// [`Arg::default_value_ifs`] only using [`OsStr`]s instead.
    /// [`Arg::default_value_ifs`]: ./struct.Arg.html#method.default_value_ifs
    /// [`OsStr`]: https://doc.rust-lang.org/std/ffi/struct.OsStr.html
    #[cfg_attr(feature = "lints", allow(explicit_counter_loop))]
    pub fn default_value_ifs_os<T>(mut self, ifs: &[(T, Option<&'help OsStr>, &'help OsStr)]) -> Self where T: Hash {
        for &(arg, val, default) in ifs {
            self = self.default_value_if_os(arg, val, default);
        }
        self
    }

    /// Specifies that if the value is not passed in as an argument, that it should be retrieved
    /// from the environment, if available. If it is not present in the environment, then default
    /// rules will apply.
    ///
    /// **NOTE:** If the user *does not* use this argument at runtime, [`ArgMatches::occurrences_of`]
    /// will return `0` even though the [`ArgMatches::value_of`] will return the default specified.
    ///
    /// **NOTE:** If the user *does not* use this argument at runtime [`ArgMatches::is_present`] will
    /// return `true` if the variable is present in the environment . If you wish to determine whether
    /// the argument was used at runtime or not, consider [`ArgMatches::occurrences_of`] which will
    /// return `0` if the argument was *not* used at runtime.
    ///
    /// **NOTE:** This implicitly sets [`Arg::takes_value(true)`].
    ///
    /// **NOTE:** If [`Arg::multiple(true)`] is set then [`Arg::use_delimiter(true)`] should also be
    /// set. Otherwise, only a single argument will be returned from the environment variable. The
    /// default delimiter is `,` and follows all the other delimiter rules.
    ///
    /// # Examples
    ///
    /// In this example, we show the variable coming from the environment:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{App, Arg};
    ///
    /// env::set_var("MY_FLAG", "env");
    ///
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG"))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("flag"), Some("env"));
    /// ```
    ///
    /// In this example, we show the variable coming from an option on the CLI:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{App, Arg};
    ///
    /// env::set_var("MY_FLAG", "env");
    ///
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG"))
    ///     .get_matches_from(vec![
    ///         "prog", "--flag", "opt"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("flag"), Some("opt"));
    /// ```
    ///
    /// In this example, we show the variable coming from the environment even with the
    /// presence of a default:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{App, Arg};
    ///
    /// env::set_var("MY_FLAG", "env");
    ///
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG")
    ///         .default_value("default"))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("flag"), Some("env"));
    /// ```
    ///
    /// In this example, we show the use of multiple values in a single environment variable:
    ///
    /// ```rust
    /// # use std::env;
    /// # use clap::{App, Arg};
    ///
    /// env::set_var("MY_FLAG_MULTI", "env1,env2");
    ///
    /// let m = App::new("prog")
    ///     .arg(Arg::new("flag")
    ///         .long("flag")
    ///         .env("MY_FLAG_MULTI")
    ///         .multiple(true)
    ///         .use_delimiter(true))
    ///     .get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert_eq!(m.values_of("flag").unwrap().collect::<Vec<_>>(), vec!["env1", "env2"]);
    /// ```
    pub fn env(self, name: &'help str) -> Self { self.env_os(OsStr::new(name)) }

    /// Specifies that if the value is not passed in as an argument, that it should be retrieved
    /// from the environment if available in the exact same manner as [`Arg::env`] only using
    /// [`OsStr`]s instead.
    pub fn env_os(mut self, name: &'help OsStr) -> Self {
        self.env = Some((name, env::var_os(name)));
        self
    }

    /// Allows custom ordering of args within the help message. Args with a lower value will be
    /// displayed first in the help message. This is helpful when one would like to emphasise
    /// frequently used args, or prioritize those towards the top of the list. Duplicate values
    /// **are** allowed. Args with duplicate display orders will be displayed in alphabetical
    /// order.
    ///
    /// **NOTE:** The default is 999 for all arguments.
    ///
    /// **NOTE:** This setting is ignored for [positional arguments] which are always displayed in
    /// [index] order.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("a") // Typically args are grouped alphabetically by name.
    ///                              // Args without a display_order have a value of 999 and are
    ///                              // displayed alphabetically with all other 999 valued args.
    ///         .long("long-option")
    ///         .short('o')
    ///         .takes_value(true)
    ///         .help("Some help and text"))
    ///     .arg(Arg::new("b")
    ///         .long("other-option")
    ///         .short('O')
    ///         .takes_value(true)
    ///         .display_order(1)   // In order to force this arg to appear *first*
    ///                             // all we have to do is give it a value lower than 999.
    ///                             // Any other args with a value of 1 will be displayed
    ///                             // alphabetically with this one...then 2 values, then 3, etc.
    ///         .help("I should be first!"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays the following help message
    ///
    /// ```notrust
    /// cust-ord
    ///
    /// USAGE:
    ///     cust-ord [FLAGS] [OPTIONS]
    ///
    /// FLAGS:
    ///     -h, --help       Prints help information
    ///     -V, --version    Prints version information
    ///
    /// OPTIONS:
    ///     -O, --other-option <b>    I should be first!
    ///     -o, --long-option <a>     Some help and text
    /// ```
    /// [positional arguments]: ./struct.Arg.html#method.index
    /// [index]: ./struct.Arg.html#method.index
    pub fn display_order(mut self, ord: usize) -> Self {
        self.disp_ord = ord;
        self
    }

    /// Specifies that this arg is the last, or final, positional argument (i.e. has the highest
    /// index) and is *only* able to be accessed via the `--` syntax (i.e. `$ prog args --
    /// last_arg`). Even, if no other arguments are left to parse, if the user omits the `--` syntax
    /// they will receive an [`UnknownArgument`] error. Setting an argument to `.last(true)` also
    /// allows one to access this arg early using the `--` syntax. Accessing an arg early, even with
    /// the `--` syntax is otherwise not possible.
    ///
    /// **NOTE:** This will change the usage string to look like `$ prog [FLAGS] [-- <ARG>]` if
    /// `ARG` is marked as `.last(true)`.
    ///
    /// **NOTE:** This setting will imply [`AppSettings::DontCollapseArgsInUsage`] because failing
    /// to set this can make the usage string very confusing.
    ///
    /// **NOTE**: This setting only applies to positional arguments, and has no affect on FLAGS /
    /// OPTIONS
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::TakesValue`]
    ///
    /// **CAUTION:** Using this setting *and* having child subcommands is not
    /// recommended with the exception of *also* using [`AppSettings::ArgsNegateSubcommands`]
    /// (or [`AppSettings::SubcommandsNegateReqs`] if the argument marked `Last` is also
    /// marked [`ArgSettings::Required`])
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Arg, ArgSettings};
    /// Arg::new("args")
    ///     .setting(ArgSettings::Last)
    /// # ;
    /// ```
    ///
    /// Setting [`Last`] ensures the arg has the highest [index] of all positional args
    /// and requires that the `--` syntax be used to access it early.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("first"))
    ///     .arg(Arg::new("second"))
    ///     .arg(Arg::new("third")
    ///         .setting(ArgSettings::Last))
    ///     .try_get_matches_from(vec![
    ///         "prog", "one", "--", "three"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// let m = res.unwrap();
    /// assert_eq!(m.value_of("third"), Some("three"));
    /// assert!(m.value_of("second").is_none());
    /// ```
    ///
    /// Even if the positional argument marked `Last` is the only argument left to parse,
    /// failing to use the `--` syntax results in an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("first"))
    ///     .arg(Arg::new("second"))
    ///     .arg(Arg::new("third")
    ///         .setting(ArgSettings::Last))
    ///     .try_get_matches_from(vec![
    ///         "prog", "one", "two", "three"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::UnknownArgument);
    /// ```
    /// [index]: ./struct.Arg.html#method.index
    /// [`AppSettings::DontCollapseArgsInUsage`]: ./enum.AppSettings.html#variant.DontCollapseArgsInUsage
    /// [`AppSettings::ArgsNegateSubcommands`]: ./enum.AppSettings.html#variant.ArgsNegateSubcommands
    /// [`AppSettings::SubcommandsNegateReqs`]: ./enum.AppSettings.html#variant.SubcommandsNegateReqs
    /// [`ArgSettings::Required`]: ./enum.ArgSetings.html#variant.Required
    /// [`UnknownArgument`]: ./enum.ErrorKind.html#variant.UnknownArgument
    pub fn last(self, l: bool) -> Self {
        if l {
            self.setting(ArgSettings::Last)
        } else {
            self.unset_setting(ArgSettings::Last)
        }
    }

    /// Specifies that the argument is required by default. Required by default means it is
    /// required, when no other conflicting rules or overrides have been evaluated. Conflicting
    /// rules take precedence over being required.
    ///
    /// **Pro tip:** Flags (i.e. not positional, or arguments that take values) shouldn't be
    /// required by default. This is because if a flag were to be required, it should simply be
    /// implied. No additional information is required from user. Flags by their very nature are
    /// simply boolean on/off switches. The only time a user *should* be required to use a flag
    /// is if the operation is destructive in nature, and the user is essentially proving to you,
    /// "Yes, I know what I'm doing."
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Arg, ArgSettings};
    /// Arg::new("config")
    ///     .setting(ArgSettings::Required)
    /// # ;
    /// ```
    ///
    /// Setting [`Required`] requires that the argument be used at runtime.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .settings(&[ArgSettings::Required, ArgSettings::TakesValue])
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Not setting [`Required`] and then *not* supplying that argument at runtime is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings, ErrorKind};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .settings(&[ArgSettings::Required, ArgSettings::TakesValue])
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::MissingRequiredArgument);
    /// ```
    /// [`Required`]: ./enum.ArgSettings.html#variant.Required
    pub fn required(self, r: bool) -> Self {
        if r {
            self.setting(ArgSettings::Required)
        } else {
            self.unset_setting(ArgSettings::Required)
        }
    }

    /// Specifies that the argument takes a value at run time.
    ///
    /// **NOTE:** values for arguments may be specified in any of the following methods
    ///
    /// * Using a space such as `-o value` or `--option value`
    /// * Using an equals and no space such as `-o=value` or `--option=value`
    /// * Use a short and no space such as `-ovalue`
    ///
    /// **NOTE:** By default, args which allow [multiple values] are delimited by commas, meaning
    /// `--option=val1,val2,val3` is three values for the `--option` argument. If you wish to
    /// change the delimiter to another character you can use [`Arg::value_delimiter(char)`],
    /// alternatively you can turn delimiting values **OFF** by using
    /// [`Arg::unset_setting(ArgSettings::UseValueDelimiter`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("config")
    ///     .setting(ArgSettings::TakesValue)
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .setting(ArgSettings::TakesValue))
    ///     .get_matches_from(vec![
    ///         "prog", "--mode", "fast"
    ///     ]);
    ///
    /// assert!(m.is_present("mode"));
    /// assert_eq!(m.value_of("mode"), Some("fast"));
    /// ```
    /// [`Arg::value_delimiter(char)`]: ./struct.Arg.html#method.value_delimiter
    /// [`Arg::unset_setting(ArgSettings::UseValueDelimiter`]: ./enum.ArgSettings.html#variant.UseValueDelimiter
    /// [multiple values]: ./enum.ArgSettings.html#variant.MultipleValues
    pub fn takes_value(self, tv: bool) -> Self {
        if tv {
            self.setting(ArgSettings::TakesValue)
        } else {
            self.unset_setting(ArgSettings::TakesValue)
        }
    }

    /// Allows values which start with a leading hyphen (`-`)
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::TakesValue`]
    ///
    /// **WARNING**: Take caution when using this setting combined with
    /// [`ArgSettings::MultipleValues`], as this becomes ambiguous `$ prog --arg -- -- val`. All
    /// three `--, --, val` will be values when the user may have thought the second `--` would
    /// constitute the normal, "Only positional args follow" idiom. To fix this, consider using
    /// [`ArgSettings::MultipleOccurrences`] which only allows a single value at a time.
    ///
    /// **WARNING**: When building your CLIs, consider the effects of allowing leading hyphens and
    /// the user passing in a value that matches a valid short. For example `prog -opt -F` where
    /// `-F` is supposed to be a value, yet `-F` is *also* a valid short for another arg. Care should
    /// should be taken when designing these args. This is compounded by the ability to "stack"
    /// short args. I.e. if `-val` is supposed to be a value, but `-v`, `-a`, and `-l` are all valid
    /// shorts.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Arg, ArgSettings};
    /// Arg::new("pattern")
    ///     .setting(ArgSettings::AllowHyphenValues)
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("pat")
    ///         .setting(ArgSettings::AllowHyphenValues)
    ///         .long("pattern"))
    ///     .get_matches_from(vec![
    ///         "prog", "--pattern", "-file"
    ///     ]);
    ///
    /// assert_eq!(m.value_of("pat"), Some("-file"));
    /// ```
    ///
    /// Not setting [`Arg::allow_hyphen_values(true)`] and supplying a value which starts with a
    /// hyphen is an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("pat")
    ///         .setting(ArgSettings::TakesValue)
    ///         .long("pattern"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--pattern", "-file"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::UnknownArgument);
    /// ```
    /// [`ArgSettings::AllowHyphenValues`]: ./enum.ArgSettings.html#variant.AllowHyphenValues
    /// [`ArgSettings::MultipleValues`]: ./enum.ArgSettings.html#variant.MultipleValues
    /// [`ArgSettings::MultipleOccurrences`]: ./enum.ArgSettings.html#variant.MultipleOccurrences
    /// [`Arg::number_of_values(1)`]: ./struct.Arg.html#method.number_of_values
    pub fn allow_hyphen_values(self, a: bool) -> Self {
        if a {
            self.setting(ArgSettings::AllowHyphenValues)
        } else {
            self.unset_setting(ArgSettings::AllowHyphenValues)
        }
    }

    /// Requires that options use the `--option=val` syntax (i.e. an equals between the option and
    /// associated value) **Default:** `false`
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::TakesValue`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{Arg, ArgSettings};
    /// Arg::new("config")
    ///     .long("config")
    ///     .setting(ArgSettings::RequireEquals)
    /// # ;
    /// ```
    ///
    /// Setting [`RequireEquals`] requires that the option have an equals sign between
    /// it and the associated value.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .setting(ArgSettings::RequireEquals)
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config=file.conf"
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// Setting [`RequireEquals`] and *not* supplying the equals will cause an error
    /// unless [`ArgSettings::EmptyValues`] is set.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .setting(ArgSettings::RequireEquals)
    ///         .long("config"))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config", "file.conf"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::EmptyValue);
    /// ```
    /// [`RequireEquals`]: ./enum.ArgSettings.html#variant.RequireEquals
    /// [`ArgSettings::EmptyValues`]: ./enum.ArgSettings.html#variant.EmptyValues
    /// [`ArgSettings::EmptyValues`]: ./enum.ArgSettings.html#variant.TakesValue
    pub fn require_equals(mut self, r: bool) -> Self {
        if r {
            self.unsetb(ArgSettings::AllowEmptyValues);
            self.setting(ArgSettings::RequireEquals)
        } else {
            self.unset_setting(ArgSettings::RequireEquals)
        }
    }

    /// Specifies that an argument can be matched to all child [``]s.
    ///
    /// **NOTE:** Global arguments *only* propagate down, **not** up (to parent commands), however
    /// their values once a user uses them will be propagated back up to parents. In effect, this
    /// means one should *define* all global arguments at the top level, however it doesn't matter
    /// where the user *uses* the global argument.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("debug")
    ///     .short('d')
    ///     .setting(ArgSettings::Global)
    /// # ;
    /// ```
    ///
    /// For example, assume an appliction with two subcommands, and you'd like to define a
    /// `--verbose` flag that can be called on any of the subcommands and parent, but you don't
    /// want to clutter the source with three duplicate [`Arg`] definitions.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("verb")
    ///         .long("verbose")
    ///         .short('v')
    ///         .setting(ArgSettings::Global))
    ///     .subcommand(App::new("test"))
    ///     .subcommand(App::new("do-stuff"))
    ///     .get_matches_from(vec![
    ///         "prog", "do-stuff", "--verbose"
    ///     ]);
    ///
    /// assert_eq!(m.subcommand_name(), Some("do-stuff"));
    /// let sub_m = m.subcommand_matches("do-stuff").unwrap();
    /// assert!(sub_m.is_present("verb"));
    /// ```
    /// [``]: ./struct.App.html#method.subcommand
    /// [required]: ./enum.ArgSettings.html#variant.Required
    /// [`ArgMatches`]: ./struct.ArgMatches.html
    /// [`ArgMatches::is_present("flag")`]: ./struct.ArgMatches.html#method.is_present
    /// [`Arg`]: ./struct.Arg.html
    pub fn global(self, g: bool) -> Self {
        if g {
            self.setting(ArgSettings::Global)
        } else {
            self.unset_setting(ArgSettings::Global)
        }
    }

    /// Specifies that *multiple values* may only be set using the delimiter. This means if an
    /// if an option is encountered, and no delimiter is found, it automatically assumed that no
    /// additional values for that option follow. This is unlike the default, where it is generally
    /// assumed that more values will follow regardless of whether or not a delimiter is used.
    ///
    /// **NOTE:** The default is `false`.
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::UseValueDelimiter`] and
    /// [`ArgSettings::TakesValue`]
    ///
    /// **NOTE:** It's a good idea to inform the user that use of a delimiter is required, either
    /// through help text or other means.
    ///
    /// # Examples
    ///
    /// These examples demonstrate what happens when `require_delimiter(true)` is used. Notice
    /// everything works in this first example, as we use a delimiter, as expected.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let delims = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .short('o')
    ///         .settings(&[ArgSettings::RequireDelimiter, ArgSettings::MultipleValues]))
    ///     .get_matches_from(vec![
    ///         "prog", "-o", "val1,val2,val3",
    ///     ]);
    ///
    /// assert!(delims.is_present("opt"));
    /// assert_eq!(delims.values_of("opt").unwrap().collect::<Vec<_>>(), ["val1", "val2", "val3"]);
    /// ```
    /// In this next example, we will *not* use a delimiter. Notice it's now an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .short('o')
    ///         .setting(ArgSettings::RequireDelimiter))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-o", "val1", "val2", "val3",
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// let err = res.unwrap_err();
    /// assert_eq!(err.kind, ErrorKind::UnknownArgument);
    /// ```
    /// What's happening is `-o` is getting `val1`, and because delimiters are required yet none
    /// were present, it stops parsing `-o`. At this point it reaches `val2` and because no
    /// positional arguments have been defined, it's an error of an unexpected argument.
    ///
    /// In this final example, we contrast the above with `clap`'s default behavior where the above
    /// is *not* an error.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let delims = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .short('o')
    ///         .setting(ArgSettings::MultipleValues))
    ///     .get_matches_from(vec![
    ///         "prog", "-o", "val1", "val2", "val3",
    ///     ]);
    ///
    /// assert!(delims.is_present("opt"));
    /// assert_eq!(delims.values_of("opt").unwrap().collect::<Vec<_>>(), ["val1", "val2", "val3"]);
    /// ```
    /// [`ArgSettings::UseValueDelimiter`]: ./enum.ArgSettings.html#variant.UseValueDelimiter
    /// [`ArgSettings::TakesValue`]: ./enum.ArgSettings.html#variant.TakesValue
    pub fn require_delimiter(mut self, d: bool) -> Self {
        if d {
            self.setb(ArgSettings::UseValueDelimiter);
            self.unsetb(ArgSettings::ValueDelimiterNotSet);
            self.setb(ArgSettings::UseValueDelimiter);
            self.setting(ArgSettings::RequireDelimiter)
        } else {
            self.unsetb(ArgSettings::UseValueDelimiter);
            self.unsetb(ArgSettings::UseValueDelimiter);
            self.unset_setting(ArgSettings::RequireDelimiter)
        }
    }

    /// Specifies if the possible values of an argument should be displayed in the help text or
    /// not. Defaults to `false` (i.e. show possible values)
    ///
    /// This is useful for args with many values, or ones which are explained elsewhere in the
    /// help text.
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::TakesValue`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("config")
    ///     .setting(ArgSettings::HidePossibleValues)
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("mode")
    ///         .long("mode")
    ///         .possible_values(&["fast", "slow"])
    ///         .setting(ArgSettings::HidePossibleValues));
    /// ```
    /// If we were to run the above program with `--help` the `[values: fast, slow]` portion of
    /// the help text would be omitted.
    pub fn hide_possible_values(self, hide: bool) -> Self {
        if hide {
            self.setting(ArgSettings::HidePossibleValues)
        } else {
            self.unset_setting(ArgSettings::HidePossibleValues)
        }
    }

    /// Specifies that the default value of an argument should not be displayed in the help text.
    ///
    /// This is useful when default behavior of an arg is explained elsewhere in the help text.
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::TakesValue`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("config")
    ///     .setting(ArgSettings::HideDefaultValue)
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("connect")
    ///     .arg(Arg::new("host")
    ///         .long("host")
    ///         .default_value("localhost")
    ///         .setting(ArgSettings::HideDefaultValue));
    ///
    /// ```
    ///
    /// If we were to run the above program with `--help` the `[default: localhost]` portion of
    /// the help text would be omitted.
    pub fn hide_default_value(self, hide: bool) -> Self {
        if hide {
            self.setting(ArgSettings::HideDefaultValue)
        } else {
            self.unset_setting(ArgSettings::HideDefaultValue)
        }
    }

    /// Allows an argument to accept explicitly empty values. An empty value must be specified at
    /// the command line with an explicit `""`, `''`, or `--option=`
    ///
    /// **NOTE:** By default empty values are *not* allowed
    ///
    /// **NOTE:** Implicitly sets [`ArgSettings::TakesValue`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("file")
    ///     .long("file")
    ///     .setting(ArgSettings::AllowEmptyValues)
    /// # ;
    /// ```
    /// The default is to *not* allow empty values.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .short('v')
    ///         .setting(ArgSettings::TakesValue))
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config="
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::EmptyValue);
    /// ```
    /// By adding this setting, we can allow empty values
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .short('v')
    ///         .setting(ArgSettings::AllowEmptyValues)) // implies TakesValue
    ///     .try_get_matches_from(vec![
    ///         "prog", "--config="
    ///     ]);
    ///
    /// assert!(res.is_ok());
    /// assert_eq!(res.unwrap().value_of("config"), None);
    /// ```
    /// [`ArgSettings::TakesValue`]: ./enum.ArgSettings.html#variant.TakesValue
    pub fn empty_values(mut self, ev: bool) -> Self {
        if ev {
            self.setting(ArgSettings::AllowEmptyValues)
        } else {
            self = self.setting(ArgSettings::TakesValue);
            self.unset_setting(ArgSettings::AllowEmptyValues)
        }
    }

    /// Hides an argument from help message output.
    ///
    /// **NOTE:** This does **not** hide the argument from usage strings on error
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("debug")
    ///     .setting(ArgSettings::Hidden)
    /// # ;
    /// ```
    /// Setting `Hidden` will hide the argument when displaying help text
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .setting(ArgSettings::Hidden)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [FLAGS]
    ///
    /// FLAGS:
    /// -h, --help       Prints help information
    /// -V, --version    Prints version information
    /// ```
    pub fn hidden(self, h: bool) -> Self {
        if h {
            self.setting(ArgSettings::Hidden)
        } else {
            self.unset_setting(ArgSettings::Hidden)
        }
    }

    /// When used with [`Arg::possible_values`] it allows the argument value to pass validation even
    /// if the case differs from that of the specified `possible_value`.
    ///
    /// **Pro Tip:** Use this setting with [`arg_enum!`]
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::TakesValue`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// # use std::ascii::AsciiExt;
    /// let m = App::new("pv")
    ///     .arg(Arg::new("option")
    ///         .long("--option")
    ///         .setting(ArgSettings::IgnoreCase)
    ///         .possible_value("test123"))
    ///     .get_matches_from(vec![
    ///         "pv", "--option", "TeSt123",
    ///     ]);
    ///
    /// assert!(m.value_of("option").unwrap().eq_ignore_ascii_case("test123"));
    /// ```
    ///
    /// This setting also works when multiple values can be defined:
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("pv")
    ///     .arg(Arg::new("option")
    ///         .short('o')
    ///         .long("--option")
    ///         .settings(&[ArgSettings::IgnoreCase, ArgSettings::MultipleValues])
    ///         .possible_value("test123")
    ///         .possible_value("test321"))
    ///     .get_matches_from(vec![
    ///         "pv", "--option", "TeSt123", "teST123", "tESt321"
    ///     ]);
    ///
    /// let matched_vals = m.values_of("option").unwrap().collect::<Vec<_>>();
    /// assert_eq!(&*matched_vals, &["TeSt123", "teST123", "tESt321"]);
    /// ```
    /// [`arg_enum!`]: ./macro.arg_enum.html
    pub fn case_insensitive(self, ci: bool) -> Self {
        if ci {
            self.setting(ArgSettings::IgnoreCase)
        } else {
            self.unset_setting(ArgSettings::IgnoreCase)
        }
    }

    /// Specifies that an argument should allow grouping of multiple values via a
    /// delimiter. I.e. should `--option=val1,val2,val3` be parsed as three values (`val1`, `val2`,
    /// and `val3`) or as a single value (`val1,val2,val3`). Defaults to using `,` (comma) as the
    /// value delimiter for all arguments that accept values (options and positional arguments)
    ///
    /// **NOTE:** When this setting is used, it will default [`Arg::value_delimiter`]
    /// to the comma `,`.
    ///
    /// **NOTE:** Implicitly sets [`ArgSettings::TakesValue`]
    ///
    /// # Examples
    ///
    /// The following example shows the default behavior.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let delims = App::new("prog")
    ///     .arg(Arg::new("option")
    ///         .long("option")
    ///         .setting(ArgSettings::UseValueDelimiter)
    ///         .takes_value(true))
    ///     .get_matches_from(vec![
    ///         "prog", "--option=val1,val2,val3",
    ///     ]);
    ///
    /// assert!(delims.is_present("option"));
    /// assert_eq!(delims.occurrences_of("option"), 1);
    /// assert_eq!(delims.values_of("option").unwrap().collect::<Vec<_>>(), ["val1", "val2", "val3"]);
    /// ```
    /// The next example shows the difference when turning delimiters off. This is the default
    /// behavior
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let nodelims = App::new("prog")
    ///     .arg(Arg::new("option")
    ///         .long("option")
    ///         .setting(ArgSettings::TakesValue))
    ///     .get_matches_from(vec![
    ///         "prog", "--option=val1,val2,val3",
    ///     ]);
    ///
    /// assert!(nodelims.is_present("option"));
    /// assert_eq!(nodelims.occurrences_of("option"), 1);
    /// assert_eq!(nodelims.value_of("option").unwrap(), "val1,val2,val3");
    /// ```
    /// [`Arg::value_delimiter`]: ./struct.Arg.html#method.value_delimiter
    pub fn use_delimiter(mut self, d: bool) -> Self {
        if d {
            if self.val_delim.is_none() {
                self.val_delim = Some(',');
            }
            self.setb(ArgSettings::TakesValue);
            self.setb(ArgSettings::UseValueDelimiter);
            self.unset_setting(ArgSettings::ValueDelimiterNotSet)
        } else {
            self.val_delim = None;
            self.unsetb(ArgSettings::UseValueDelimiter);
            self.unset_setting(ArgSettings::ValueDelimiterNotSet)
        }
    }

    /// Specifies that any values inside the associated ENV variables of an argument should not be
    /// displayed in the help text.
    ///
    /// This is useful when ENV vars contain sensitive values.
    ///
    /// **NOTE:** Setting this implies [`ArgSettings::TakesValue`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("config")
    ///     .setting(ArgSettings::HideDefaultValue)
    /// # ;
    /// ```
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("connect")
    ///     .arg(Arg::new("host")
    ///         .long("host")
    ///         .env("CONNECT")
    ///         .setting(ArgSettings::HideEnvValues));
    ///
    /// ```
    ///
    /// If we were to run the above program with `$ CONNECT=super_secret connect --help` the
    /// `[default: CONNECT=super_secret]` portion of the help text would be omitted.
    pub fn hide_env_values(self, hide: bool) -> Self {
        if hide {
            self.setting(ArgSettings::HideEnvValues)
        } else {
            self.unset_setting(ArgSettings::HideEnvValues)
        }
    }

    /// When set to `true` the help string will be displayed on the line after the argument and
    /// indented once. This can be helpful for arguments with very long or complex help messages.
    /// This can also be helpful for arguments with very long flag names, or many/long value names.
    ///
    /// **NOTE:** To apply this setting to all arguments consider using
    /// [`AppSettings::NextLineHelp`] on the entire `App`
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("opt")
    ///         .long("long-option-flag")
    ///         .short('o')
    ///         .settings(&[ArgSettings::TakesValue, ArgSettings::NextLineHelp])
    ///         .value_names(&["value1", "value2"])
    ///         .help("Some really long help and complex\n\
    ///                help that makes more sense to be\n\
    ///                on a line after the option"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays the following help message
    ///
    /// ```notrust
    /// nlh
    ///
    /// USAGE:
    ///     nlh [FLAGS] [OPTIONS]
    ///
    /// FLAGS:
    ///     -h, --help       Prints help information
    ///     -V, --version    Prints version information
    ///
    /// OPTIONS:
    ///     -o, --long-option-flag <value1> <value2>
    ///         Some really long help and complex
    ///         help that makes more sense to be
    ///         on a line after the option
    /// ```
    /// [`AppSettings::NextLineHelp`]: ./enum.AppSettings.html#variant.NextLineHelp
    pub fn next_line_help(mut self, nlh: bool) -> Self {
        if nlh {
            self.setb(ArgSettings::NextLineHelp);
        } else {
            self.unsetb(ArgSettings::NextLineHelp);
        }
        self
    }

    /// A convienience method for setting both `Arg::multiple_occurrences(true)` and
    /// `Arg::multiple_values(true)`
    pub fn multiple(mut self, multi: bool) -> Self {
        if multi {
            self.setb(ArgSettings::MultipleOccurrences);
            self.setting(ArgSettings::MultipleValues)
        } else {
            self.unsetb(ArgSettings::MultipleOccurrences);
            self.unset_setting(ArgSettings::MultipleValues)
        }
    }

    /// Specifies that an argument accepts multiple values in a single occurrence. However, without
    /// any additional settings, this argument may not be used more than once.
    ///
    /// For example, `--opt val1 val2` is allowed, but `--opt val1 val2 --opt val3` is not.
    ///
    /// **NOTE:** Implicitly sets [`Arg::takes_value(true)`]
    ///
    /// By default, with no other settings being used, `clap` will stop parsing values if any of the
    /// following are true:
    ///
    /// * It finds another flag or option (i.e. something that starts with a `-`)
    ///   * This has the exception of if the current argument accepts values that start with a hyphen
    /// * It finds a valid [subcommand]
    /// * The equals sign was used (`$ prog --option=value`)
    /// * A [delimiter] was used (`$ prog --option value1,value2`
    ///
    /// **WARNING:**
    ///
    /// Setting `Arg::multiple_values(true)` for an argument with no other details can be dangerous
    /// in some circumstances. Because multiple values are allowed, `--option val1 val2 val3` is
    /// perfectly valid yet imagine `val3` was supposed to be a positional argument, or subcommand.
    /// Be careful when designing a CLI where positional arguments or subcommands are *also*
    /// expected.
    ///
    /// **WARNING:**
    ///
    /// When using args with `Arg::multiple_values(true)` *and* [subcommands], one should consider
    /// the posibility of an argument value being the same as a valid subcommand.
    ///
    /// By default `clap` will parse the value/subcommand as a value.
    ///
    /// As an example, consider a CLI with an option `--ui-paths=<paths>...` and subcommand `signer`
    ///
    /// The following would be parsed as values to `--ui-paths`.
    ///
    /// ```notrust
    /// $ program --ui-paths path1 path2 signer
    /// ```
    ///
    /// However, `clap` will parse `signer` as a subcommand in all of these cases:
    ///
    /// ```notrust
    /// $ program --ui-paths=path1,path2 signer
    /// $ program --ui-paths=path1 signer
    /// $ program --ui-paths path1,path2 signer
    /// ```
    ///
    /// We could also add additional parameters to `--ui-paths` to solve this issue. Consider adding
    /// [`Arg::number_of_values(1)`] or using *only* [`MultipleOccurrences`]. The following are all
    /// valid, and `signer` is parsed as a subcommand.
    ///
    /// ```notrust
    /// $ program --ui-paths path1 signer
    /// $ program --ui-paths path1 --ui-paths signer signer
    /// ```
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("debug")
    ///     .short('d')
    ///     .setting(ArgSettings::MultipleValues)
    /// # ;
    /// ```
    ///
    /// An example with options
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .multiple_values(true) // implies TakesValue
    ///         .short('F'))
    ///     .get_matches_from(vec![
    ///         "prog", "-F", "file1", "file2", "file3"
    ///     ]);
    ///
    /// assert!(m.is_present("file"));
    ///
    /// assert_eq!(m.occurrences_of("file"), 1); // notice only one occurrence
    ///
    /// let files: Vec<_> = m.values_of("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3"]);
    /// ```
    /// Although `Arg::multiple_values(true)` has been specified, we cannot use the argument more
    /// than once.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .multiple_values(true) // implies TakesValue
    ///         .short('F'))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1", "-F", "file2", "-F", "file3"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::UnexpectedMultipleUsage)
    /// ```
    ///
    /// A common mistake is to define an option which allows multiple values, and a positional
    /// argument.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .multiple_values(true) // implies TakesValue
    ///         .short('F'))
    ///     .arg(Arg::new("word")
    ///         .index(1))
    ///     .get_matches_from(vec![
    ///         "prog", "-F", "file1", "file2", "file3", "word"
    ///     ]);
    ///
    /// assert!(m.is_present("file"));
    ///
    /// let files: Vec<_> = m.values_of("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3", "word"]); // wait...what?!
    ///
    /// assert!(!m.is_present("word")); // but we clearly used word!
    /// ```
    ///
    /// The problem is `clap` doesn't know when to stop parsing values for "files". This is further
    /// compounded by if we'd said `word -F file1 file2` it would have worked fine, so it would
    /// appear to only fail sometimes...not good!
    ///
    /// A solution for the example above is to limit how many values with a [maxium], or [specific]
    /// number, or to say **only** [`Arg::multiple_occurrences(true)`] but multiple values per
    /// occurrence is not.
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .multiple_occurrences(true) // *does not* imply takes_value(true)
    ///         .takes_value(true)
    ///         .short('F'))
    ///     .arg(Arg::new("word")
    ///         .index(1))
    ///     .get_matches_from(vec![
    ///         "prog", "-F", "file1", "-F", "file2", "-F", "file3", "word"
    ///     ]);
    ///
    /// assert!(m.is_present("file"));
    /// let files: Vec<_> = m.values_of("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3"]);
    /// assert!(m.is_present("word"));
    /// assert_eq!(m.value_of("word"), Some("word"));
    /// ```
    /// For completeness sake let's fix the above error and get a pretty message to the user :)
    ///
    /// ```rust
    /// # use clap::{App, Arg, ErrorKind, ArgSettings};
    /// let res = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .multiple_occurrences(true) // *does not* imply takes_value(true)
    ///         .takes_value(true)
    ///         .short('F'))
    ///     .arg(Arg::new("word")
    ///         .index(1))
    ///     .try_get_matches_from(vec![
    ///         "prog", "-F", "file1", "file2", "file3", "word"
    ///     ]);
    ///
    /// assert!(res.is_err());
    /// assert_eq!(res.unwrap_err().kind, ErrorKind::UnknownArgument);
    /// ```
    ///
    /// [option]: ./enum.ArgSettings.html#variant.TakesValue
    /// [options]: ./enum.ArgSettings.html#variant.TakesValue
    /// [subcommands]: ./struct.App.html#method.subcommand
    /// [subcommand]: ./struct.App.html#method.subcommand
    /// [positionals]: ./struct.Arg.html#method.index
    /// [`Arg::number_of_values(1)`]: ./struct.Arg.html#method.number_of_values
    /// [`MultipleOccurrences`]: ./enum.ArgSettings.html#variant.MultipleOccurrences
    /// [`MultipleValues`]: ./enum.ArgSettings.html#variant.MultipleValues
    /// [maximum number of values]: ./struct.Arg.html#method.max_values
    /// [specific number of values]: ./struct.Arg.html#method.number_of_values
    /// [maximum]: ./struct.Arg.html#method.max_values
    /// [specific]: ./struct.Arg.html#method.number_of_values
    /// [value terminator]: ./struct.Arg.html#method.value_terminator
    pub fn multiple_values(self, multi: bool) -> Self {
        if multi {
            self.setting(ArgSettings::MultipleValues)
        } else {
            self.unset_setting(ArgSettings::MultipleValues)
        }
    }

    /// Specifies that the argument may appear more than once.
    /// For flags, this results
    /// in the number of occurrences of the flag being recorded. For example `-ddd` or `-d -d -d`
    /// would count as three occurrences. For options or arguments that take a value, this
    /// *does not* affect how many values they can accept. (i.e. only one at a time is allowed)
    ///
    /// For example, `--opt val1 --opt val2` is allowed, but `--opt val1 val2` is not.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// Arg::new("debug")
    ///     .short('d')
    ///     .setting(ArgSettings::MultipleOccurrences)
    /// # ;
    /// ```
    /// An example with flags
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("verbose")
    ///         .setting(ArgSettings::MultipleOccurrences)
    ///         .short('v'))
    ///     .get_matches_from(vec![
    ///         "prog", "-v", "-v", "-v"    // note, -vvv would have same result
    ///     ]);
    ///
    /// assert!(m.is_present("verbose"));
    /// assert_eq!(m.occurrences_of("verbose"), 3);
    /// ```
    ///
    /// An example with options
    ///
    /// ```rust
    /// # use clap::{App, Arg, ArgSettings};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("file")
    ///         .settings(&[ArgSettings::MultipleOccurrences, ArgSettings::TakesValue])
    ///         .short('F'))
    ///     .get_matches_from(vec![
    ///         "prog", "-F", "file1", "-F", "file2", "-F", "file3"
    ///     ]);
    ///
    /// assert!(m.is_present("file"));
    /// assert_eq!(m.occurrences_of("file"), 3);
    /// let files: Vec<_> = m.values_of("file").unwrap().collect();
    /// assert_eq!(files, ["file1", "file2", "file3"]);
    /// ```
    /// [option]: ./enum.ArgSettings.html#variant.TakesValue
    /// [options]: ./enum.ArgSettings.html#variant.TakesValue
    /// [subcommands]: ./struct.App.html#method.subcommand
    /// [positionals]: ./struct.Arg.html#method.index
    /// [`Arg::number_of_values(1)`]: ./struct.Arg.html#method.number_of_values
    /// [`MultipleOccurrences`]: ./enum.ArgSettings.html#variant.MultipleOccurrences
    /// [`MultipleValues`]: ./enum.ArgSettings.html#variant.MultipleValues
    /// [maximum number of values]: ./struct.Arg.html#method.max_values
    /// [specific number of values]: ./struct.Arg.html#method.number_of_values
    /// [maximum]: ./struct.Arg.html#method.max_values
    /// [specific]: ./struct.Arg.html#method.number_of_values
    pub fn multiple_occurrences(self, multi: bool) -> Self {
        if multi {
            self.setting(ArgSettings::MultipleOccurrences)
        } else {
            self.unset_setting(ArgSettings::MultipleOccurrences)
        }
    }

    // @TODO remove?
    /// Indicates that all parameters passed after this should not be parsed
    /// individually, but rather passed in their entirety. It is worth noting
    /// that setting this requires all values to come after a `--` to indicate they
    /// should all be captured. For example:
    ///
    /// ```notrust
    /// --foo something -- -v -v -v -b -b -b --baz -q -u -x
    /// ```
    /// Will result in everything after `--` to be considered one raw argument. This behavior
    /// may not be exactly what you are expecting and using [`AppSettings::TrailingVarArg`]
    /// may be more appropriate.
    ///
    /// **NOTE:** Implicitly sets [`Arg::multiple(true)`], [`Arg::allow_hyphen_values(true)`], and
    /// [`Arg::last(true)`] when set to `true`
    ///
    /// [`Arg::multiple(true)`]: ./struct.Arg.html#method.multiple
    /// [`Arg::allow_hyphen_values(true)`]: ./struct.Arg.html#method.allow_hyphen_values
    /// [`Arg::last(true)`]: ./struct.Arg.html#method.last
    /// [`AppSettings::TrailingVarArg`]: ./enum.AppSettings.html#variant.TrailingVarArg
    pub fn raw(self, raw: bool) -> Self { self.multiple(raw).allow_hyphen_values(raw).last(raw) }

    /// Hides an argument from short help message output.
    ///
    /// **NOTE:** This does **not** hide the argument from usage strings on error
    ///
    /// **NOTE:** Setting this option will cause next-line-help output style to be used
    /// when long help (`--help`) is called.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("debug")
    ///     .hidden_short_help(true)
    /// # ;
    /// ```
    /// Setting `hidden_short_help(true)` will hide the argument when displaying short help text
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hidden_short_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "-h"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [FLAGS]
    ///
    /// FLAGS:
    /// -h, --help       Prints help information
    /// -V, --version    Prints version information
    /// ```
    ///
    /// However, when --help is called
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hidden_short_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// Then the following would be displayed
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [FLAGS]
    ///
    /// FLAGS:
    ///     --config     Some help text describing the --config arg
    /// -h, --help       Prints help information
    /// -V, --version    Prints version information
    /// ```
    pub fn hidden_short_help(self, hide: bool) -> Self {
        if hide {
            self.setting(ArgSettings::HiddenShortHelp)
        } else {
            self.unset_setting(ArgSettings::HiddenShortHelp)
        }
    }

    /// Hides an argument from long help message output.
    ///
    /// **NOTE:** This does **not** hide the argument from usage strings on error
    ///
    /// **NOTE:** Setting this option will cause next-line-help output style to be used
    /// when long help (`--help`) is called.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// Arg::new("debug")
    ///     .hidden_long_help(true)
    /// # ;
    /// ```
    /// Setting `hidden_long_help(true)` will hide the argument when displaying long help text
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hidden_long_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "--help"
    ///     ]);
    /// ```
    ///
    /// The above example displays
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [FLAGS]
    ///
    /// FLAGS:
    /// -h, --help       Prints help information
    /// -V, --version    Prints version information
    /// ```
    ///
    /// However, when -h is called
    ///
    /// ```rust
    /// # use clap::{App, Arg};
    /// let m = App::new("prog")
    ///     .arg(Arg::new("cfg")
    ///         .long("config")
    ///         .hidden_long_help(true)
    ///         .help("Some help text describing the --config arg"))
    ///     .get_matches_from(vec![
    ///         "prog", "-h"
    ///     ]);
    /// ```
    ///
    /// Then the following would be displayed
    ///
    /// ```notrust
    /// helptest
    ///
    /// USAGE:
    ///    helptest [FLAGS]
    ///
    /// FLAGS:
    ///     --config     Some help text describing the --config arg
    /// -h, --help       Prints help information
    /// -V, --version    Prints version information
    /// ```
    pub fn hidden_long_help(self, hide: bool) -> Self {
        if hide {
            self.setting(ArgSettings::HiddenLongHelp)
        } else {
            self.unset_setting(ArgSettings::HiddenLongHelp)
        }
    }

    // @TODO @docs @v3-beta: write better docs as ArgSettings is now critical
    /// Checks if one of the [`ArgSettings`] is set for the argument
    /// [`ArgSettings`]: ./enum.ArgSettings.html
    pub fn is_set(&self, s: ArgSettings) -> bool { self.settings.is_set(s) }

    /// Sets one of the [`ArgSettings`] settings for the argument
    /// [`ArgSettings`]: ./enum.ArgSettings.html
    pub fn setting(mut self, s: ArgSettings) -> Self {
        self.setb(s);
        self
    }

    // @TODO @docs @v3-beta: write better docs as ArgSettings is now critical
    /// Sets multiple [`ArgSettings`] for the argument
    /// [`ArgSettings`]: ./enum.ArgSettings.html
    pub fn settings(mut self, settings: &[ArgSettings]) -> Self {
        for s in settings {
            self.settings.set(*s);
        }
        self
    }

    /// Unsets one of the [`ArgSettings`] for the argument
    /// [`ArgSettings`]: ./enum.ArgSettings.html
    pub fn unset_setting(mut self, s: ArgSettings) -> Self {
        self.unsetb(s);
        self
    }

    /// Set a custom heading for this arg to be printed under
    pub fn help_heading(mut self, s: Option<&'help str>) -> Self {
        self.help_heading = s;
        self
    }

    fn set_default_delimiter(&mut self) {
        if (self.is_set(ArgSettings::UseValueDelimiter)
            || self.is_set(ArgSettings::RequireDelimiter))
            && self.val_delim.is_none()
        {
            self.val_delim = Some(',');
        }
    }

    #[doc(hidden)]
    pub fn _build(&mut self) {
        self.set_default_delimiter();
        if self.is_positional() {
            if self.max_vals.is_some()
                || self.min_vals.is_some()
                || (self.num_vals.is_some() && self.num_vals.unwrap() > 1)
            {
                self.setb(ArgSettings::MultipleValues);
                self.setb(ArgSettings::MultipleOccurrences);
            }
        } else if self.is_set(ArgSettings::TakesValue) {
            if let Some(ref vec) = self.val_names {
                if vec.len() > 1 {
                    self.num_vals = Some(vec.len() as u64);
                }
            }
        }
    }

    // @TODO @p6 @naming @internal: rename to set_mut
    #[doc(hidden)]
    pub fn setb(&mut self, s: ArgSettings) { self.settings.set(s); }

    // @TODO @p6 @naming @internal: rename to unset_mut
    #[doc(hidden)]
    pub fn unsetb(&mut self, s: ArgSettings) { self.settings.unset(s); }

    #[doc(hidden)]
    pub fn has_switch(&self) -> bool { self.key.has_switch() }

    // We should probably figure out this the hard way in the help display, or find a better way
    // to encapsulate this.
//    #[doc(hidden)]
//    pub fn longest_filter(&self) -> bool {
//        self.is_set(ArgSettings::TakesValue) || self.long.is_some() || self.short.is_none()
//    }

    // Used for positionals when printing
    #[doc(hidden)]
    pub fn multiple_str(&self) -> &str {
        let mult_vals = self
            .val_names
            .as_ref()
            .map_or(true, |names| names.len() < 2);
        if (self.is_set(ArgSettings::MultipleValues)
            || self.is_set(ArgSettings::MultipleOccurrences))
            && mult_vals
        {
            "..."
        } else {
            ""
        }
    }

    pub fn name_no_brackets(&self) -> &str {
        assert!(self.index.is_some());
        self.val_names.as_ref().expect(INTERNAL_ERROR_MSG).get(0).expect(INTERNAL_ERROR_MSG)
    }
    pub fn is_flag(&self) -> bool {
        self.value.is_none() && self.key.has_switch()
    }
    pub fn is_option(&self) -> bool {
        self.value.is_some() && self.key.has_switch()
    }
    pub fn is_positional(&self) -> bool {
        self.value.is_some() && !self.key.has_switch()
    }
}

impl<'help, 'z> From<&'z Arg<'help>> for Arg<'help> {
    fn from(a: &'z Arg<'help>) -> Self { a.clone() }
}

impl<'help> From<&'help str> for Arg<'help> {
    fn from(s: &'help str) -> Self { UsageParser::from_usage(s).parse() }
}

impl<'help> PartialEq for Arg<'help> {
    fn eq(&self, other: &Arg<'help>) -> bool { self.id == other.id }
}

impl<'help> Display for Arg<'help> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.key.is_positional() {
            // Positional
            let mut delim = String::new();
            delim.push(if self.is_set(ArgSettings::RequireDelimiter) {
                self.val_delim.expect(INTERNAL_ERROR_MSG)
            } else {
                ' '
            });
            if let Some(ref names) = self.val_names {
                write!(
                    f,
                    "{}",
                    names
                        .values()
                        .map(|n| format!("<{}>", n))
                        .collect::<Vec<_>>()
                        .join(&*delim)
                )?;
            } else {
                write!(f, "<{}>", self.val_names.as_ref().expect(INTERNAL_ERROR_MSG).get(0).expect(INTERNAL_ERROR_MSG))?;
            }
            if self.settings.is_set(ArgSettings::MultipleValues)
                && (self.val_names.is_none()
                || (self.val_names.is_some() && self.val_names.as_ref().unwrap().len() == 1))
            {
                write!(f, "...")?;
            }
            return Ok(());
        } else if !self.is_set(ArgSettings::TakesValue) {
            // Flag
            if let Some(l) = self.long {
                write!(f, "--{}", l)?;
            } else if let Some(s) = self.short {
                write!(f, "-{}", s)?;
            }

            return Ok(());
        }
        let sep = if self.is_set(ArgSettings::RequireEquals) {
            "="
        } else {
            " "
        };
        // Write the name such --long or -l
        if let Some(l) = self.long {
            write!(f, "--{}{}", l, sep)?;
        } else {
            write!(f, "-{}{}", self.short.unwrap(), sep)?;
        }
        let delim = if self.is_set(ArgSettings::RequireDelimiter) {
            self.val_delim.expect(INTERNAL_ERROR_MSG)
        } else {
            ' '
        };

        // Write the values such as <name1> <name2>
        if let Some(ref vec) = self.val_names {
            let mut it = vec.iter().peekable();
            while let Some((_, val)) = it.next() {
                write!(f, "<{}>", val)?;
                if it.peek().is_some() {
                    write!(f, "{}", delim)?;
                }
            }
            let num = vec.len();
            if self.is_set(ArgSettings::MultipleValues) && num == 1 {
                write!(f, "...")?;
            }
        } else if let Some(num) = self.num_vals {
            let mut it = (0..num).peekable();
            while let Some(_) = it.next() {
                write!(f, "<{}>", self.val_names.as_ref().expect(INTERNAL_ERROR_MSG).get(0).expect(INTERNAL_ERROR_MSG))?;
                if it.peek().is_some() {
                    write!(f, "{}", delim)?;
                }
            }
            if self.is_set(ArgSettings::MultipleValues) && num == 1 {
                write!(f, "...")?;
            }
        } else {
            write!(
                f,
                "<{}>{}",
                self.val_names.as_ref().expect(INTERNAL_ERROR_MSG).get(0).expect(INTERNAL_ERROR_MSG),
                if self.is_set(ArgSettings::MultipleOccurrences) {
                    "..."
                } else {
                    ""
                }
            )?;
        }

        Ok(())
    }
}

impl<'help> Eq for Arg<'help> {}

impl<'help> fmt::Debug for Arg<'help> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "Arg {{ \
                id: {:?}, \
                help: {:?}, \
                long_help: {:?}, \
                conflicts_with: {:?}, \
                settings: {:?}, \
                required_unless: {:?}, \
                overrides_with: {:?},  \
                requires: {:?}, \
                requires_ifs: {:?}, \
                key: {:?}, \
                index: {:?}, \
                possible_values: {:?}, \
                value_names: {:?}, \
                number_of_values: {:?}, \
                max_values: {:?}, \
                min_values: {:?}, \
                value_delimiter: {:?}, \
                default_value_ifs: {:?}, \
                value_terminator: {:?}, \
                display_order: {:?}, \
                env: {:?}, \
                unified_ord: {:?}, \
                default_value: {:?}, \
                validator: {}, \
                validator_os: {} \
             }}",
            self.id,
            self.help,
            self.long_help,
            self.blacklist,
            self.settings,
            self.r_unless,
            self.overrides,
            self.requires,
            self.r_ifs,
            self.key,
            self.index,
            self.possible_vals,
            self.val_names,
            self.num_vals,
            self.max_vals,
            self.min_vals,
            self.val_delim,
            self.default_vals_ifs,
            self.terminator,
            self.disp_ord,
            self.env,
            self.unified_ord,
            self.default_val,
            self.validator.as_ref().map_or("None", |_| "Some(Fn)"),
            self.validator_os.as_ref().map_or("None", |_| "Some(Fn)")
        )
    }
}

#[cfg(feature = "yaml")]
impl<'help> From<&'help yaml_rust::Yaml> for Arg<'help> {
    /// Creates a new instance of [`Arg`] from a .yml (YAML) file.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # #[macro_use]
    /// # extern crate clap;
    /// # use clap::Arg;
    /// # fn main() {
    /// let yml = load_yaml!("arg.yml");
    /// let arg = Arg::from_yaml(yml);
    /// # }
    /// ```
    /// [`Arg`]: ./struct.Arg.html
    fn from(y: &'help yaml_rust::yaml::Hash) -> Arg<'help> {
        // We WANT this to panic on error...so expect() is good.
        let name_yml = y.keys().nth(0).unwrap();
        let name_str = name_yml.as_str().unwrap();
        let mut a = Arg::new(name_str);
        let arg_settings = y.get(name_yml).unwrap().as_hash().unwrap();

        for (k, v) in arg_settings.iter() {
            a = match k.as_str().unwrap() {
                "short" => yaml_to_char!(a, v, short),
                "long" => yaml_to_str!(a, v, long),
                "aliases" => yaml_vec_or_str!(v, a, alias),
                "help" => yaml_to_str!(a, v, help),
                "long_help" => yaml_to_str!(a, v, long_help),
                "required" => yaml_to_bool!(a, v, required),
                "required_if" => yaml_tuple2!(a, v, required_if),
                "required_ifs" => yaml_tuple2!(a, v, required_if),
                "takes_value" => yaml_to_bool!(a, v, takes_value),
                "index" => yaml_to_u64!(a, v, index),
                "global" => yaml_to_bool!(a, v, global),
                "multiple" => yaml_to_bool!(a, v, multiple),
                "hidden" => yaml_to_bool!(a, v, hidden),
                "next_line_help" => yaml_to_bool!(a, v, next_line_help),
                "empty_values" => yaml_to_bool!(a, v, empty_values),
                "number_of_values" => yaml_to_u64!(a, v, number_of_values),
                "max_values" => yaml_to_u64!(a, v, max_values),
                "min_values" => yaml_to_u64!(a, v, min_values),
                "value_name" => yaml_to_str!(a, v, value_name),
                "use_delimiter" => yaml_to_bool!(a, v, use_delimiter),
                "allow_hyphen_values" => yaml_to_bool!(a, v, allow_hyphen_values),
                "require_delimiter" => yaml_to_bool!(a, v, require_delimiter),
                "value_delimiter" => yaml_to_str!(a, v, value_delimiter),
                "required_unless" => yaml_to_str!(a, v, required_unless),
                "display_order" => yaml_to_usize!(a, v, display_order),
                "default_value" => yaml_to_str!(a, v, default_value),
                "default_value_if" => yaml_tuple3!(a, v, default_value_if),
                "default_value_ifs" => yaml_tuple3!(a, v, default_value_if),
                "env" => yaml_to_str!(a, v, env),
                "value_names" => yaml_vec_or_str!(v, a, value_name),
                "requires" => yaml_vec_or_str!(v, a, requires),
                "requires_if" => yaml_tuple2!(a, v, requires_if),
                "requires_ifs" => yaml_tuple2!(a, v, requires_if),
                "conflicts_with" => yaml_vec_or_str!(v, a, conflicts_with),
                "overrides_with" => yaml_vec_or_str!(v, a, overrides_with),
                "possible_values" => yaml_vec_or_str!(v, a, possible_value),
                "required_unless_one" => yaml_vec_or_str!(v, a, required_unless),
                "required_unless_all" => {
                    a = yaml_vec_or_str!(v, a, required_unless);
                    a.setb(ArgSettings::RequiredUnlessAll);
                    a
                }
                s => panic!(
                    "Unknown Arg setting '{}' in YAML file for arg '{}'",
                    s, name_str
                ),
            }
        }

        a
    }
}

// Flags
#[cfg(test)]
mod test {
    use build::ArgSettings;
    use util::VecMap;

    use super::Arg;

    #[test]
    fn flag_display() {
        let mut f = Arg::new("flg");
        f.settings.set(ArgSettings::MultipleOccurrences);
        f.long = Some("flag");

        assert_eq!(&*format!("{}", f), "--flag");

        let mut f2 = Arg::new("flg");
        f2.short = Some('f');

        assert_eq!(&*format!("{}", f2), "-f");
    }

    #[test]
    fn flag_display_single_alias() {
        let mut f = Arg::new("flg");
        f.long = Some("flag");
        f.aliases = Some(vec![("als", true)]);

        assert_eq!(&*format!("{}", f), "--flag")
    }

    #[test]
    fn flag_display_multiple_aliases() {
        let mut f = Arg::new("flg");
        f.short = Some('f');
        f.aliases = Some(vec![
            ("alias_not_visible", false),
            ("f2", true),
            ("f3", true),
            ("f4", true),
        ]);
        assert_eq!(&*format!("{}", f), "-f");
    }

    // Options

    #[test]
    fn option_display1() {
        let o = Arg::new("opt")
            .long("option")
            .takes_value(true)
            .multiple(true);

        assert_eq!(&*format!("{}", o), "--option <opt>...");
    }

    #[test]
    fn option_display2() {
        let o2 = Arg::new("opt")
            .short('o')
            .value_names(&["file", "name"]);

        assert_eq!(&*format!("{}", o2), "-o <file> <name>");
    }

    #[test]
    fn option_display3() {
        let o2 = Arg::new("opt")
            .short('o')
            .multiple(true)
            .value_names(&["file", "name"]);

        assert_eq!(&*format!("{}", o2), "-o <file> <name>");
    }

    #[test]
    fn option_display_single_alias() {
        let o = Arg::new("opt")
            .takes_value(true)
            .long("option")
            .visible_alias("als");

        assert_eq!(&*format!("{}", o), "--option <opt>");
    }

    #[test]
    fn option_display_multiple_aliases() {
        let o = Arg::new("opt")
            .long("option")
            .takes_value(true)
            .visible_aliases(&["als2", "als3", "als4"])
            .alias("als_not_visible");

        assert_eq!(&*format!("{}", o), "--option <opt>");
    }

    // Positionals

    #[test]
    fn positiona_display_mult() {
        let mut p = Arg::new("pos").index(1);
        p.setb(ArgSettings::MultipleValues);

        assert_eq!(&*format!("{}", p), "<pos>...");
    }

    #[test]
    fn positional_display_required() {
        let mut p2 = Arg::new("pos").index(1);
        p2.settings.set(ArgSettings::Required);

        assert_eq!(&*format!("{}", p2), "<pos>");
    }

    #[test]
    fn positional_display_val_names() {
        let mut p2 = Arg::new("pos").index(1);
        let mut vm = VecMap::new();
        vm.insert(0, "file1");
        vm.insert(1, "file2");
        p2.val_names = Some(vm);

        assert_eq!(&*format!("{}", p2), "<file1> <file2>");
    }

    #[test]
    fn positional_display_val_names_req() {
        let mut p2 = Arg::new("pos").index(1);
        p2.settings.set(ArgSettings::Required);
        let mut vm = VecMap::new();
        vm.insert(0, "file1");
        vm.insert(1, "file2");
        p2.val_names = Some(vm);

        assert_eq!(&*format!("{}", p2), "<file1> <file2>");
    }
}
