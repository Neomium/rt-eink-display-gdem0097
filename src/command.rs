/// Guarantees each command is stored as a u8
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Command {
    DriverOutputControl = 0x01,

    DeepSleepMode = 0x10,

    DataEntryModeSetting = 0x11,

    SoftwareReset = 0x12,

    TemperatureSensorControl = 0x18,

    MasterActivation = 0x20,

    DisplayUpdateControl1 = 0x21,

    DisplayUpdateControl2 = 0x22,

    WriteRam = 0x24,

    /// Writes the reference image used by partial refreshes.
    WritePreviousRam = 0x26,

    BorderWaveformControl = 0x3C,

    SetRamXAddressStartEnd = 0x44,

    SetRamYAddressStartEnd = 0x45,

    SetRamXAddressCounter = 0x4E,

    SetRamYAddressCounter = 0x4F,
}
