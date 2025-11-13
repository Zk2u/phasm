use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Day {
    pub fn name(&self) -> &str {
        match self {
            Day::Monday => "Mon",
            Day::Tuesday => "Tue",
            Day::Wednesday => "Wed",
            Day::Thursday => "Thu",
            Day::Friday => "Fri",
            Day::Saturday => "Sat",
            Day::Sunday => "Sun",
        }
    }

    pub fn all() -> &'static [Day] {
        &[
            Day::Monday,
            Day::Tuesday,
            Day::Wednesday,
            Day::Thursday,
            Day::Friday,
            Day::Saturday,
            Day::Sunday,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(pub u8, pub u8); // hour, minute

impl Time {
    pub fn new(hour: u8, minute: u8) -> Self {
        assert!(hour < 24 && minute < 60);
        Time(hour, minute)
    }

    pub fn to_mins(&self) -> u16 {
        self.0 as u16 * 60 + self.1 as u16
    }

    pub fn from_mins(m: u16) -> Self {
        Time((m / 60) as u8, (m % 60) as u8)
    }

    pub fn add(&self, mins: u16) -> Self {
        Self::from_mins(self.to_mins() + mins)
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange(pub Time, pub Time);

impl TimeRange {
    pub fn new(start: Time, end: Time) -> Self {
        assert!(start < end);
        TimeRange(start, end)
    }

    pub fn contains(&self, t: Time) -> bool {
        t >= self.0 && t < self.1
    }

    pub fn can_fit(&self, start: Time, dur: u16) -> bool {
        self.contains(start) && start.add(dur) <= self.1
    }
}

impl fmt::Display for TimeRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AptType {
    Cleaning,
    Checkup,
    Filling,
    RootCanal,
}

impl AptType {
    pub fn dur(&self) -> u16 {
        match self {
            AptType::Cleaning => 15,
            AptType::Checkup => 30,
            AptType::Filling => 45,
            AptType::RootCanal => 60,
        }
    }

    pub fn price(&self) -> f32 {
        match self {
            AptType::Cleaning => 50.0,
            AptType::Checkup => 75.0,
            AptType::Filling => 150.0,
            AptType::RootCanal => 200.0,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            AptType::Cleaning => "Cleaning",
            AptType::Checkup => "Checkup",
            AptType::Filling => "Filling",
            AptType::RootCanal => "Root Canal",
        }
    }

    pub fn all() -> &'static [AptType] {
        &[
            AptType::Cleaning,
            AptType::Checkup,
            AptType::Filling,
            AptType::RootCanal,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Slot {
    pub day: Day,
    pub time: Time,
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.day.name(), self.time)
    }
}

#[derive(Debug, Clone)]
pub struct ConfirmedBooking {
    pub user_id: u64,
    pub name: String,
    pub email: String,
    pub apt_type: AptType,
    pub amount_paid: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReqStatus {
    AwaitingPreauth,
    PreauthSuccess,
    SlotConfirmed,
    SlotTaken,
    NoSlot,
}

#[derive(Debug, Clone)]
pub struct PendingReq {
    pub user_id: u64,
    pub name: String,
    pub email: String,
    pub slot: Option<Slot>,
    pub apt_type: AptType,
    pub status: ReqStatus,
}
