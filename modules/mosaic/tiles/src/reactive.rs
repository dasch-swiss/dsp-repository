use leptos::prelude::*;

/// A reactive binding wrapper that can take any value and upgrade it to a RwSignal (default)
/// or a reactive_stores::Field (Store, ArcStore, Field, ArcField and Subfield can be converted).
///
/// For example:
/// ```rs
/// <Checkbox checked=true />
/// ```
/// is effectively the same as:
/// ```rs
/// <Checkbox checked=RwSignal::new(true) />
/// ```
/// A RwSignal defined elsewhere can also be used:
/// ```rs
/// let checked = RwSignal::new(true);
///
/// view!{
///     <Checkbox checked />
/// }
/// ```
/// In this case the `RwSignal` and the `Checkbox` are coupled. Changing one will update the other
/// and notify all listeners.
#[derive(Clone, Debug)]
pub enum Reactive<T>
where
    T: Send + Sync + Clone + 'static,
    RwSignal<T>: Send + Sync + Clone + Copy + 'static,
{
    RwSignal(RwSignal<T>),
}

impl<T> Reactive<T>
where
    T: Send + Sync + Clone + 'static,
{
    #[inline]
    pub fn get(&self) -> T {
        match self {
            Self::RwSignal(rw_signal) => rw_signal.get(),
        }
    }

    #[inline]
    pub fn get_untracked(&self) -> T {
        match self {
            Self::RwSignal(rw_signal) => rw_signal.get_untracked(),
        }
    }

    #[inline]
    pub fn set(&self, value: T) {
        match self {
            Self::RwSignal(rw_signal) => rw_signal.set(value),
        }
    }

    #[inline]
    pub fn with<K>(&self, fun: impl FnOnce(&T) -> K) -> K {
        match self {
            Self::RwSignal(rw_signal) => rw_signal.with(fun),
        }
    }

    #[inline]
    pub fn update(&self, fun: impl FnOnce(&mut T)) {
        match self {
            Self::RwSignal(rw_signal) => rw_signal.update(fun),
        }
    }
}

impl<T: Send + Sync + Clone> Copy for Reactive<T> {}

impl<T: Default + Send + Clone + Sync + 'static> Default for Reactive<T> {
    fn default() -> Self {
        Self::RwSignal(RwSignal::<T>::new(Default::default()))
    }
}

impl<T: Default + Send + Clone + Sync + 'static> Reactive<T> {
    pub fn new(value: T) -> Self {
        Self::RwSignal(RwSignal::<T>::new(value))
    }
}

impl From<&str> for Reactive<String> {
    fn from(value: &str) -> Self {
        Reactive::RwSignal(RwSignal::new(value.to_string()))
    }
}

impl<T> From<T> for Reactive<T>
where
    T: Send + Sync + Clone + 'static,
{
    fn from(value: T) -> Self {
        Reactive::RwSignal(RwSignal::new(value))
    }
}

impl<T> From<RwSignal<T>> for Reactive<T>
where
    T: Send + Sync + Clone + 'static,
{
    fn from(value: RwSignal<T>) -> Self {
        Reactive::RwSignal(value)
    }
}
