use std::{ffi, str, ptr};
use geo::LatLon;
use simconnect::DispatchResult;
use crate::aircraft::Aircraft;
use crate::sim_connection::{SimConnection, SimMessage};

#[derive(Debug)]
enum SimStringError {
    Utf8Error(str::Utf8Error),
    CStrError(ffi::FromBytesUntilNulError),
}

impl From<str::Utf8Error> for SimStringError {
    fn from(value: str::Utf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

impl From<ffi::FromBytesUntilNulError> for SimStringError {
    fn from(value: ffi::FromBytesUntilNulError) -> Self {
        Self::CStrError(value)
    }
}

/// A representation of SimConnect's strings.
///
/// It will usually be created by doing `ptr::read_unaligned(..)` in a struct
/// the string is contained in.
///
/// Example
///
///    #[repr(C, packed)]
///    struct MyStruct {
///        title: SimString<256>,
///        airport: SimString<32>,
///        // other sim data
///    }
#[derive(Clone, Debug)]
#[repr(C, packed)]
pub struct SimString<const N: usize>([u8; N]);

impl<const N: usize> SimString<N> {
    fn to_string(&self) -> Result<String, SimStringError> {
        let bytes = self.0;
        let c_str = ffi::CStr::from_bytes_until_nul(&bytes)?;
        Ok(String::from(c_str.to_str()?))
    }
}

impl<const N: usize> std::fmt::Display for SimString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string().unwrap())
    }
}


/// All the data we want to fetch from the sim
// #[derive(Debug)]
// rust adds padding to the the struct, pack it to avoid adding extra nulls
#[repr(C, packed)]
struct RawSimData {
    // https://docs.flightsimulator.com/html/Programming_Tools/SimVars/Aircraft_SimVars/Aircraft_Misc_Variables.htm
    title: SimString<128>,
    eng_combustion_1: f64,
    eng_combustion_2: f64,
    eng_combustion_3: f64,
    eng_combustion_4: f64,
    latitude: f64,
    longitude: f64,
    // https://docs.flightsimulator.com/html/Programming_Tools/SimVars/Miscellaneous_Variables.htm
    sim_on_ground: f64,
    // https://docs.flightsimulator.com/html/Programming_Tools/SimVars/Aircraft_SimVars/Aircraft_RadioNavigation_Variables.htm
    // may or may not contain aircraft registration
    atc_id: SimString<32>,
}

impl TryFrom<RawSimData> for Aircraft {
    type Error = SimStringError;

    fn try_from(raw: RawSimData) -> Result<Self, Self::Error> {
        Ok(Self {
            title: raw.title.to_string()?,
            // FIXME: ICAO isn't available from simconnect yet.
            //
            // a possible option is to do a lookup for the aircraft (using the title)
            // in the Community & Official folders, finding the aircraft.cfg and fetching
            // icao_type_designator
            icao: String::from("N/A"),
            position: LatLon::from_radians(raw.latitude, raw.longitude),
            // not the most reliable source, but its the best we have
            registration: raw.atc_id.to_string()?,
            engines_on: [
                raw.eng_combustion_1 != 0.0,
                raw.eng_combustion_2 != 0.0,
                raw.eng_combustion_3 != 0.0,
                raw.eng_combustion_4 != 0.0,
            ],
            on_ground: raw.sim_on_ground != 0.0,
        })
    }
}

pub struct Msfs(simconnect::SimConnector);

impl Msfs {
    pub fn connect() -> Self {
        let mut conn = simconnect::SimConnector::new();
        conn.connect("Logbook");
        conn.add_data_definition(
            0,
            "TITLE",
            "",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_STRING128,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "ENG COMBUSTION:1",
            "Boolean",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "ENG COMBUSTION:2",
            "Boolean",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "ENG COMBUSTION:3",
            "Boolean",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "ENG COMBUSTION:4",
            "Boolean",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "PLANE LATITUDE",
            "Radians",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "PLANE LONGITUDE",
            "Radians",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "SIM ON GROUND",
            "Boolean",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_FLOAT64,
            u32::MAX,
            0.0
        );
        conn.add_data_definition(
            0,
            "ATC ID",
            "",
            simconnect::SIMCONNECT_DATATYPE_SIMCONNECT_DATATYPE_STRING32,
            u32::MAX,
            0.0
        );

        // receive data related to the user aircraft
        conn.request_data_on_sim_object(
            0, // request id
            0, // define id
            0, // object id (user)
            simconnect::SIMCONNECT_PERIOD_SIMCONNECT_PERIOD_SECOND,
            0, // flags
            0, // origin
            0, // interval
            0, // limit
        );
        Self(conn)
    }
}

impl SimConnection for Msfs {
    type Error = String;

    fn next_message(&self) -> Result<crate::sim_connection::SimMessage, Self::Error> {
        let msg = match self.0.get_next_message()? {
            DispatchResult::Open(_) => SimMessage::Open,
            DispatchResult::Quit(_) => SimMessage::Quit,
            DispatchResult::SimObjectData(data) => unsafe {
                if data.dwDefineID == 0 {
                    let sim_data_ptr = ptr::addr_of!(data.dwData) as *const RawSimData;
                    let sim_data_value = ptr::read_unaligned(sim_data_ptr);
                    // fixme: unwrap
                    let aircraft = Aircraft::try_from(sim_data_value).unwrap();
                    SimMessage::SimData(aircraft)
                } else {
                    // fixme: return more info
                    SimMessage::Unknown
                }
            },
            msg => {
                println!("Unhandled message: {msg:?}");
                SimMessage::Unknown
            },
        };
        Ok(msg)
    }
}
