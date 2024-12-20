use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::i2c::I2c;

pub use crate::internal::measurement::Measurement;
pub use crate::internal::scd4x::I2C_ADDRESS;

use crate::error::Error;
use crate::internal::crc::{crc8, crc8_verify_chunked_3};
use crate::internal::scd4x::{
    decode_serial_number, Command, GET_AUTOMATIC_SELF_CALIBRATION_ENABLED,
    GET_AUTOMATIC_SELF_CALIBRATION_TARGET, GET_DATA_READY_STATUS, GET_SENSOR_ALTITUDE,
    GET_SERIAL_NUMBER, GET_TEMPERATURE_OFFSET, PERFORM_FACTORY_RESET, PERFORM_FORCED_RECALIBRATION,
    PERFORM_SELF_TEST, PERSIST_SETTINGS, READ_MEASUREMENT, REINIT, SET_AMBIENT_PRESSURE,
    SET_AUTOMATIC_SELF_CALIBRATION_ENABLED, SET_AUTOMATIC_SELF_CALIBRATION_TARGET,
    SET_SENSOR_ALTITUDE, SET_TEMPERATURE_OFFSET, START_LOW_POWER_PERIODIC_MEASUREMENT,
    START_PERIODIC_MEASUREMENT, STOP_PERIODIC_MEASUREMENT,
};

#[cfg(feature = "scd41")]
use crate::internal::scd4x::{
    GET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD, GET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD,
    MEASURE_SINGLE_SHOT, MEASURE_SINGLE_SHOT_RHT_ONLY, POWER_DOWN,
    SET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD, SET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD,
    WAKE_UP,
};

/// Driver implementation for the SCD40 CO2 sensor. This driver is compatible
/// with SCD41 devices, though it does not expose the SCD41 additional APIs
///
/// This sensor needs to be enabled via the `scd40` feature flag
#[cfg(feature = "scd40")]
pub struct Scd40<I2C, D> {
    inner: Scd4x<I2C, D>,
}

#[cfg(feature = "scd40")]
impl<I2C, D, E> Scd40<I2C, D>
where
    I2C: I2c<Error = E>,
    D: DelayNs,
{
    /// Create a new sensor using the provided I2C bus and delay implementation
    pub fn new(i2c: I2C, delay: D) -> Self {
        Self {
            inner: Scd4x::new(i2c, delay),
        }
    }

    /// Release the I2C bus held by this sensor
    pub fn release(self) -> I2C {
        self.inner.release()
    }

    /// Start periodic measurement mode. The signal update interval is 5 seconds.
    pub async fn start_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.inner.start_periodic_measurement().await
    }

    /// Stop periodic measurement mode to change the sensor configuration or
    /// to save power. Note that the sensor will only respond to other
    /// commands 500 ms after the `stop_periodic_measurement()` command
    /// has been issued.
    pub async fn stop_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.inner.stop_periodic_measurement().await
    }

    /// Start low power periodic measurement mode, signal update interval
    /// is approximately 30 seconds.
    pub async fn start_low_power_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.inner.start_low_power_periodic_measurement().await
    }

    /// Check if there is a measurement data ready to be read
    pub async fn data_ready(&mut self) -> Result<bool, Error<E>> {
        self.inner.data_ready().await
    }

    /// Read sensor output.
    ///
    /// The measurement data can only be read out  once per signal update
    /// interval as the buffer is emptied upon read-out. If no data is
    /// available in the buffer, the sensor returns a NACK. To avoid a
    /// NACK response, the `data_ready()` method can be issued to check
    /// data status.
    pub async fn read_measurement(&mut self) -> Result<Measurement, Error<E>> {
        self.inner.read_measurement().await
    }

    /// Configure the temperature offset
    pub async fn set_temperature_offset(&mut self, offset: f32) -> Result<(), Error<E>> {
        self.inner.set_temperature_offset(offset).await
    }

    /// Retrieve the configured temperature offset
    pub async fn get_temperature_offset(&mut self) -> Result<f32, Error<E>> {
        self.inner.get_temperature_offset().await
    }

    /// Reading and writing the sensor altitude must be done while the SCD4x
    /// is in idle mode. Typically, the sensor altitude is set once after
    /// device installation. To save the setting to the EEPROM, the
    /// `persist_settings()` (see Section 3.9.1) command must be issued.
    ///
    /// The default sensor altitude value is set to 0 meters above sea level.
    /// Valid input values are between 0 – 3’000 m.
    pub async fn set_sensor_altitude(&mut self, altitude: u16) -> Result<(), Error<E>> {
        self.inner.set_sensor_altitude(altitude).await
    }

    /// The `get_sensor_altitude()` command can be sent while the SCD4x
    /// is in idle mode to read out the previously saved sensor altitude
    /// value set by the `set_sensor_altitude()` command.
    pub async fn get_sensor_altitude(&mut self) -> Result<u16, Error<E>> {
        self.inner.get_sensor_altitude().await
    }

    /// The `set_ambient_pressure()` command can be sent during periodic
    /// measurements to enable continuous pressure compensation. Note that
    /// setting an ambient pressure overrides any pressure compensation
    /// based on a previously set sensor altitude. Use of this command is
    /// highly recommended for applications experiencing significant ambient
    /// pressure changes to ensure sensor accuracy. Valid input values are
    /// between 70_000 – 120_000 Pa. The default value is 101_300 Pa.
    pub async fn set_ambient_pressure(&mut self, pressure: u32) -> Result<(), Error<E>> {
        self.inner.set_ambient_pressure(pressure).await
    }

    /// Set the current state (enabled / disabled) of the ASC. By default,
    /// ASC is enabled. To save the setting to the EEPROM, the
    /// `persist_settings()` (see Section 3.9.1) command must be issued.
    pub async fn enable_automatic_self_calibration(
        &mut self,
        enabled: bool,
    ) -> Result<(), Error<E>> {
        self.inner.enable_automatic_self_calibration(enabled).await
    }

    /// The `set_automatic_self_calibration_target()` command can be sent when
    /// the SCD4x is in idle mode. It sets the value of the ASC baseline target.
    /// This is the lower-bound background CO2 concentration the sensor is exposed
    /// to regularly. The default value is 400.
    pub async fn set_automatic_self_calibration_target(
        &mut self,
        ppm_co2: u16,
    ) -> Result<(), Error<E>> {
        self.inner
            .set_automatic_self_calibration_target(ppm_co2)
            .await
    }

    /// The `get_automatic_self_calibration_target()` command can be sent when
    /// the SCD4x is in idle mode. It gets the value of the ASC baseline target.
    pub async fn get_automatic_self_calibration_target(&mut self) -> Result<u16, Error<E>> {
        self.inner.get_automatic_self_calibration_target().await
    }

    /// The `perform_forced_recalibration()` command can be sent when the SCD4x
    /// is in idle mode after having been in operation for at least 3 minutes in
    /// an environment with a homogenous and constant CO2 concentration that is
    /// already known.
    ///
    /// ppm_co2 refers to the current CO2 level.
    ///
    /// Returns either a failure or the FRC correction applied
    pub async fn perform_forced_recalibration(&mut self, ppm_co2: u16) -> Result<i16, Error<E>> {
        self.inner.perform_forced_recalibration(ppm_co2).await
    }

    /// Check if the automatic self calibration algorithm is enabled
    pub async fn get_automatic_self_calibration(&mut self) -> Result<bool, Error<E>> {
        self.inner.get_automatic_self_calibration().await
    }

    /// Configuration settings such as the temperature offset, sensor altitude
    /// and the ASC enabled/disabled parameters are by default stored in the
    /// volatile memory (RAM) only and will be lost after a power-cycle.
    /// The `persist_settings()` command  stores the current configuration in
    /// the EEPROM of the SCD4x, ensuring the settings persist across
    /// power-cycling. To avoid unnecessary wear of the EEPROM,
    /// the `persist_settings()` command should only be sent when persistence
    /// is required and if actual changes to the configuration have been made.
    pub async fn persists_settings(&mut self) -> Result<(), Error<E>> {
        self.inner.persists_settings().await
    }

    /// Reading out the serial number can be used to identify the chip
    /// and to verify the presence of the sensor.
    pub async fn serial_number(&mut self) -> Result<u64, Error<E>> {
        self.inner.serial_number().await
    }

    /// The `perform_self_test()` command can be used as an end-of-line
    /// test to check the sensor functionality.
    pub async fn perform_self_test(&mut self) -> Result<bool, Error<E>> {
        self.inner.perform_self_test().await
    }

    /// The perform_factory_reset command resets all configuration
    /// settings stored in the EEPROM and erases the FRC and ASC
    /// algorithm history.
    pub async fn perform_factory_reset(&mut self) -> Result<(), Error<E>> {
        self.inner.perform_factory_reset().await
    }

    /// The reinit command reinitializes the sensor by reloading user
    /// settings from EEPROM. Before sending the reinit command, the
    /// `stop_periodic_measurement()` command must be issued.
    /// If the reinit command does not trigger the desired
    /// re-initialization, a power-cycle should be applied to
    /// the SCD4x.
    pub async fn reinit(&mut self) -> Result<(), Error<E>> {
        self.inner.reinit().await
    }
}

/// Driver implementation for the SCD41 CO2 sensor. This driver is compatible
/// with SCD40 devices, though it exposes operations that are not available on
/// SCD40
///
/// This sensor needs to be enabled via the `scd41` feature flag
#[cfg(feature = "scd41")]
pub struct Scd41<I2C, D> {
    inner: Scd4x<I2C, D>,
}

#[cfg(feature = "scd41")]
impl<I2C, D, E> Scd41<I2C, D>
where
    I2C: I2c<Error = E>,
    D: DelayNs,
{
    /// Create a new sensor using the provided I2C bus and delay implementation
    pub fn new(i2c: I2C, delay: D) -> Self {
        Self {
            inner: Scd4x::new(i2c, delay),
        }
    }

    /// Release the I2C bus held by this sensor
    pub fn release(self) -> I2C {
        self.inner.release()
    }

    /// Start periodic measurement mode. The signal update interval is 5 seconds.
    pub async fn start_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.inner.start_periodic_measurement().await
    }

    /// Stop periodic measurement mode to change the sensor configuration or
    /// to save power. Note that the sensor will only respond to other
    /// commands 500 ms after the `stop_periodic_measurement()` command
    /// has been issued.
    pub async fn stop_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.inner.stop_periodic_measurement().await
    }

    /// Start low power periodic measurement mode, signal update interval
    /// is approximately 30 seconds.
    pub async fn start_low_power_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.inner.start_low_power_periodic_measurement().await
    }

    /// Check if there is a measurement data ready to be read
    pub async fn data_ready(&mut self) -> Result<bool, Error<E>> {
        self.inner.data_ready().await
    }

    /// Read sensor output.
    ///
    /// The measurement data can only be read out  once per signal update
    /// interval as the buffer is emptied upon read-out. If no data is
    /// available in the buffer, the sensor returns a NACK. To avoid a
    /// NACK response, the `data_ready()` method can be issued to check
    /// data status.
    pub async fn read_measurement(&mut self) -> Result<Measurement, Error<E>> {
        self.inner.read_measurement().await
    }

    /// Configure the temperature offset
    pub async fn set_temperature_offset(&mut self, offset: f32) -> Result<(), Error<E>> {
        self.inner.set_temperature_offset(offset).await
    }

    /// Retrieve the configured temperature offset
    pub async fn get_temperature_offset(&mut self) -> Result<f32, Error<E>> {
        self.inner.get_temperature_offset().await
    }

    /// Reading and writing the sensor altitude must be done while the SCD4x
    /// is in idle mode. Typically, the sensor altitude is set once after
    /// device installation. To save the setting to the EEPROM, the
    /// `persist_settings()` (see Section 3.9.1) command must be issued.
    ///
    /// The default sensor altitude value is set to 0 meters above sea level.
    /// Valid input values are between 0 – 3’000 m.
    pub async fn set_sensor_altitude(&mut self, altitude: u16) -> Result<(), Error<E>> {
        self.inner.set_sensor_altitude(altitude).await
    }

    /// The `get_sensor_altitude()` command can be sent while the SCD4x
    /// is in idle mode to read out the previously saved sensor altitude
    /// value set by the `set_sensor_altitude()` command.
    pub async fn get_sensor_altitude(&mut self) -> Result<u16, Error<E>> {
        self.inner.get_sensor_altitude().await
    }

    /// The `set_ambient_pressure()` command can be sent during periodic
    /// measurements to enable continuous pressure compensation. Note that
    /// setting an ambient pressure overrides any pressure compensation
    /// based on a previously set sensor altitude. Use of this command is
    /// highly recommended for applications experiencing significant ambient
    /// pressure changes to ensure sensor accuracy. Valid input values are
    /// between 70_000 – 120_000 Pa. The default value is 101_300 Pa.
    pub async fn set_ambient_pressure(&mut self, pressure: u32) -> Result<(), Error<E>> {
        self.inner.set_ambient_pressure(pressure).await
    }

    /// Set the current state (enabled / disabled) of the ASC. By default,
    /// ASC is enabled. To save the setting to the EEPROM, the
    /// `persist_settings()` (see Section 3.9.1) command must be issued.
    pub async fn enable_automatic_self_calibration(
        &mut self,
        enabled: bool,
    ) -> Result<(), Error<E>> {
        self.inner.enable_automatic_self_calibration(enabled).await
    }

    /// Check if the automatic self calibration algorithm is enabled
    pub async fn get_automatic_self_calibration(&mut self) -> Result<bool, Error<E>> {
        self.inner.get_automatic_self_calibration().await
    }

    /// The `set_automatic_self_calibration_target()` command can be sent when
    /// the SCD4x is in idle mode. It sets the value of the ASC baseline target.
    /// This is the lower-bound background CO2 concentration the sensor is exposed
    /// to regularly. The default value is 400.
    pub async fn set_automatic_self_calibration_target(
        &mut self,
        ppm_co2: u16,
    ) -> Result<(), Error<E>> {
        self.inner
            .set_automatic_self_calibration_target(ppm_co2)
            .await
    }

    /// The `get_automatic_self_calibration_target()` command can be sent when
    /// the SCD4x is in idle mode. It gets the value of the ASC baseline target.
    pub async fn get_automatic_self_calibration_target(&mut self) -> Result<u16, Error<E>> {
        self.inner.get_automatic_self_calibration_target().await
    }

    /// The `perform_forced_recalibration()` command can be sent when the SCD4x
    /// is in idle mode after having been in operation for at least 3 minutes in
    /// an environment with a homogenous and constant CO2 concentration that is
    /// already known.
    ///
    /// ppm_co2 refers to the current CO2 level.
    ///
    /// Returns either a failure or the FRC correction applied
    pub async fn perform_forced_recalibration(&mut self, ppm_co2: u16) -> Result<i16, Error<E>> {
        self.inner.perform_forced_recalibration(ppm_co2).await
    }

    /// Configuration settings such as the temperature offset, sensor altitude
    /// and the ASC enabled/disabled parameters are by default stored in the
    /// volatile memory (RAM) only and will be lost after a power-cycle.
    /// The `persist_settings()` command  stores the current configuration in
    /// the EEPROM of the SCD4x, ensuring the settings persist across
    /// power-cycling. To avoid unnecessary wear of the EEPROM,
    /// the `persist_settings()` command should only be sent when persistence
    /// is required and if actual changes to the configuration have been made.
    pub async fn persists_settings(&mut self) -> Result<(), Error<E>> {
        self.inner.persists_settings().await
    }

    /// Reading out the serial number can be used to identify the chip
    /// and to verify the presence of the sensor.
    pub async fn serial_number(&mut self) -> Result<u64, Error<E>> {
        self.inner.serial_number().await
    }

    /// The `perform_self_test()` command can be used as an end-of-line
    /// test to check the sensor functionality.
    pub async fn perform_self_test(&mut self) -> Result<bool, Error<E>> {
        self.inner.perform_self_test().await
    }

    /// The perform_factory_reset command resets all configuration
    /// settings stored in the EEPROM and erases the FRC and ASC
    /// algorithm history.
    pub async fn perform_factory_reset(&mut self) -> Result<(), Error<E>> {
        self.inner.perform_factory_reset().await
    }

    /// The reinit command reinitializes the sensor by reloading user
    /// settings from EEPROM. Before sending the reinit command, the
    /// `stop_periodic_measurement()` command must be issued.
    /// If the reinit command does not trigger the desired
    /// re-initialization, a power-cycle should be applied to
    /// the SCD4x.
    pub async fn reinit(&mut self) -> Result<(), Error<E>> {
        self.inner.reinit().await
    }

    /// On-demand measurement of CO2 concentration, relative humidity and
    /// temperature. The sensor output is read out by using the
    /// `read_measurement()` command (Section 3.5.2).
    pub async fn measure_single_shot(&mut self) -> Result<(), Error<E>> {
        self.inner.measure_single_shot().await
    }

    /// On-demand measurement of relative humidity and temperature only.
    /// The sensor output is read out by using the `read_measurement()`
    /// command (Section 3.5.2). CO2 output is returned as 0 ppm.
    pub async fn measure_single_shot_rht_only(&mut self) -> Result<(), Error<E>> {
        self.inner.measure_single_shot_rht_only().await
    }

    /// Put the sensor from idle to sleep to reduce current consumption.
    /// Can be used to power down when operating the sensor in
    /// power-cycled single shot mode.
    pub async fn power_down(&mut self) -> Result<(), Error<E>> {
        self.inner.power_down().await
    }

    /// Wake up the sensor from sleep mode into idle mode. Note that the
    /// SCD4x does not acknowledge the `wake_up()` command. The sensor
    /// idle state after wake up can be verified by reading out the
    /// serial number (Section 3.9.2).
    pub async fn wake_up(&mut self) -> Result<(), Error<E>> {
        self.inner.wake_up().await
    }

    pub async fn set_automatic_self_calibration_initial_period(
        &mut self,
        hours: u16,
    ) -> Result<(), Error<E>> {
        self.inner
            .set_automatic_self_calibration_initial_period(hours)
            .await
    }

    pub async fn get_automatic_self_calibration_initial_period(&mut self) -> Result<u16, Error<E>> {
        self.inner
            .get_automatic_self_calibration_initial_period()
            .await
    }

    pub async fn set_automatic_self_calibration_standard_period(
        &mut self,
        hours: u16,
    ) -> Result<(), Error<E>> {
        self.inner
            .set_automatic_self_calibration_standard_period(hours)
            .await
    }

    pub async fn get_automatic_self_calibration_standard_period(
        &mut self,
    ) -> Result<u16, Error<E>> {
        self.inner
            .get_automatic_self_calibration_standard_period()
            .await
    }
}

struct Scd4x<I2C, D> {
    i2c: I2C,
    delay: D,
    measurement_started: bool,
}

impl<I2C, D, E> Scd4x<I2C, D>
where
    I2C: I2c<Error = E>,
    D: DelayNs,
{
    fn new(i2c: I2C, delay: D) -> Self {
        Self {
            i2c,
            delay,
            measurement_started: false,
        }
    }

    fn release(self) -> I2C {
        self.i2c
    }

    async fn write_command(&mut self, cmd: Command) -> Result<(), Error<E>> {
        if self.measurement_started & !cmd.allowed_while_running {
            return Err(Error::NotAllowed);
        }

        self.i2c
            .write(I2C_ADDRESS, &cmd.op_code.to_be_bytes())
            .await
            .map_err(|e| Error::I2C(e))?;
        self.delay.delay_ms(cmd.exec_time as u32).await;

        Ok(())
    }

    async fn write_command_with_data(&mut self, cmd: Command, data: u16) -> Result<(), Error<E>> {
        if self.measurement_started & !cmd.allowed_while_running {
            return Err(Error::NotAllowed);
        }

        let c = cmd.op_code.to_be_bytes();
        let d = data.to_be_bytes();

        let mut buf = [0; 5];
        buf[0..2].copy_from_slice(&c);
        buf[2..4].copy_from_slice(&d);
        buf[4] = crc8(&d);

        self.i2c
            .write(I2C_ADDRESS, &buf)
            .await
            .map_err(|e| Error::I2C(e))?;
        self.delay.delay_ms(cmd.exec_time as u32).await;

        Ok(())
    }

    async fn write_read_command_with_data(
        &mut self,
        cmd: Command,
        data: u16,
        read_buf: &mut [u8],
    ) -> Result<(), Error<E>> {
        if self.measurement_started & !cmd.allowed_while_running {
            return Err(Error::NotAllowed);
        }

        assert_eq!(
            read_buf.len() % 3,
            0,
            "The read buffer length must be a multiple of 3"
        );

        let c = cmd.op_code.to_be_bytes();
        let d = data.to_be_bytes();

        let mut buf = [0; 5];
        buf[0..2].copy_from_slice(&c);
        buf[2..4].copy_from_slice(&d);
        buf[4] = crc8(&d);

        self.i2c
            .write(I2C_ADDRESS, &buf)
            .await
            .map_err(|e| Error::I2C(e))?;
        self.delay.delay_ms(cmd.exec_time as u32).await;

        self.i2c
            .read(I2C_ADDRESS, read_buf)
            .await
            .map_err(|e| Error::I2C(e))?;

        if !crc8_verify_chunked_3(read_buf) {
            return Err(Error::CRC);
        }

        Ok(())
    }

    async fn read_command(&mut self, cmd: Command, buf: &mut [u8]) -> Result<(), Error<E>> {
        assert_eq!(
            buf.len() % 3,
            0,
            "The buffer length must be a multiple of 3"
        );

        self.write_command(cmd).await?;
        self.i2c
            .read(I2C_ADDRESS, buf)
            .await
            .map_err(|e| Error::I2C(e))?;

        if !crc8_verify_chunked_3(buf) {
            return Err(Error::CRC);
        }

        Ok(())
    }

    async fn start_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.write_command(START_PERIODIC_MEASUREMENT).await?;
        self.measurement_started = true;
        Ok(())
    }

    async fn stop_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.write_command(STOP_PERIODIC_MEASUREMENT).await?;
        self.measurement_started = false;
        Ok(())
    }

    async fn start_low_power_periodic_measurement(&mut self) -> Result<(), Error<E>> {
        self.write_command(START_LOW_POWER_PERIODIC_MEASUREMENT)
            .await?;
        Ok(())
    }

    async fn data_ready(&mut self) -> Result<bool, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(GET_DATA_READY_STATUS, &mut buf).await?;

        let status = u16::from_be_bytes([buf[0], buf[1]]);
        Ok(status & 0x07FF != 0)
    }

    async fn read_measurement(&mut self) -> Result<Measurement, Error<E>> {
        let mut buf = [0; 9];
        self.read_command(READ_MEASUREMENT, &mut buf).await?;

        let co2 = u16::from_be_bytes([buf[0], buf[1]]);
        let temperature = u16::from_be_bytes([buf[3], buf[4]]);
        let humidity = u16::from_be_bytes([buf[6], buf[7]]);

        Ok(Measurement {
            temperature: temperature as f32 * 175.0 / 65_536.0 - 45.0,
            humidity: humidity as f32 * 100.0 / 65_536.0,
            co2,
        })
    }

    async fn set_temperature_offset(&mut self, offset: f32) -> Result<(), Error<E>> {
        let value = (offset * 65536.0 / 175.0) as i16 as u16;

        self.write_command_with_data(SET_TEMPERATURE_OFFSET, value)
            .await?;
        Ok(())
    }

    async fn get_temperature_offset(&mut self) -> Result<f32, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(GET_TEMPERATURE_OFFSET, &mut buf).await?;

        let offset = u16::from_be_bytes([buf[0], buf[1]]);
        let offset = offset as f32 * 175.0 / 65536.0;

        Ok(offset)
    }

    async fn set_sensor_altitude(&mut self, altitude: u16) -> Result<(), Error<E>> {
        if altitude > 3_000 {
            return Err(Error::InvalidInput);
        }

        self.write_command_with_data(SET_SENSOR_ALTITUDE, altitude)
            .await?;
        Ok(())
    }

    async fn get_sensor_altitude(&mut self) -> Result<u16, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(GET_SENSOR_ALTITUDE, &mut buf).await?;

        Ok(u16::from_be_bytes([buf[0], buf[1]]))
    }

    async fn set_ambient_pressure(&mut self, pressure: u32) -> Result<(), Error<E>> {
        if !(70_000..=120_000).contains(&pressure) {
            return Err(Error::InvalidInput);
        }

        let pressure = (pressure / 100) as u16;
        self.write_command_with_data(SET_AMBIENT_PRESSURE, pressure)
            .await?;
        Ok(())
    }

    async fn enable_automatic_self_calibration(&mut self, enabled: bool) -> Result<(), Error<E>> {
        self.write_command_with_data(SET_AUTOMATIC_SELF_CALIBRATION_ENABLED, enabled as u16)
            .await?;
        Ok(())
    }

    async fn get_automatic_self_calibration(&mut self) -> Result<bool, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(GET_AUTOMATIC_SELF_CALIBRATION_ENABLED, &mut buf)
            .await?;

        let raw_status = u16::from_be_bytes([buf[0], buf[1]]);
        Ok(raw_status != 0)
    }

    async fn set_automatic_self_calibration_target(
        &mut self,
        ppm_co2: u16,
    ) -> Result<(), Error<E>> {
        self.write_command_with_data(SET_AUTOMATIC_SELF_CALIBRATION_TARGET, ppm_co2)
            .await?;
        Ok(())
    }

    async fn get_automatic_self_calibration_target(&mut self) -> Result<u16, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(GET_AUTOMATIC_SELF_CALIBRATION_TARGET, &mut buf)
            .await?;

        Ok(u16::from_be_bytes([buf[0], buf[1]]))
    }

    async fn perform_forced_recalibration(&mut self, ppm_co2: u16) -> Result<i16, Error<E>> {
        let mut buf = [0; 3];
        self.write_read_command_with_data(PERFORM_FORCED_RECALIBRATION, ppm_co2, &mut buf)
            .await?;

        let result = u16::from_be_bytes([buf[0], buf[1]]);

        if result == 0xFFFF {
            return Err(Error::FrcFailed);
        }

        let frc_correction = result as i32 - 0x8000;
        Ok(frc_correction as i16)
    }

    async fn persists_settings(&mut self) -> Result<(), Error<E>> {
        self.write_command(PERSIST_SETTINGS).await?;
        Ok(())
    }

    async fn serial_number(&mut self) -> Result<u64, Error<E>> {
        let mut buf = [0; 9];
        self.read_command(GET_SERIAL_NUMBER, &mut buf).await?;

        Ok(decode_serial_number(buf))
    }

    async fn perform_self_test(&mut self) -> Result<bool, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(PERFORM_SELF_TEST, &mut buf).await?;

        let status = u16::from_be_bytes([buf[0], buf[1]]);
        Ok(status == 0)
    }

    async fn perform_factory_reset(&mut self) -> Result<(), Error<E>> {
        self.write_command(PERFORM_FACTORY_RESET).await?;
        Ok(())
    }

    async fn reinit(&mut self) -> Result<(), Error<E>> {
        self.write_command(REINIT).await?;
        Ok(())
    }

    #[cfg(feature = "scd41")]
    async fn measure_single_shot(&mut self) -> Result<(), Error<E>> {
        self.write_command(MEASURE_SINGLE_SHOT).await?;
        Ok(())
    }

    #[cfg(feature = "scd41")]
    async fn measure_single_shot_rht_only(&mut self) -> Result<(), Error<E>> {
        self.write_command(MEASURE_SINGLE_SHOT_RHT_ONLY).await?;
        Ok(())
    }

    #[cfg(feature = "scd41")]
    async fn power_down(&mut self) -> Result<(), Error<E>> {
        self.write_command(POWER_DOWN).await?;
        Ok(())
    }

    #[cfg(feature = "scd41")]
    async fn wake_up(&mut self) -> Result<(), Error<E>> {
        self.write_command(WAKE_UP).await?;
        Ok(())
    }

    #[cfg(feature = "scd41")]
    async fn set_automatic_self_calibration_initial_period(
        &mut self,
        hours: u16,
    ) -> Result<(), Error<E>> {
        self.write_command_with_data(SET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD, hours)
            .await?;
        Ok(())
    }

    #[cfg(feature = "scd41")]
    async fn get_automatic_self_calibration_initial_period(&mut self) -> Result<u16, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(GET_AUTOMATIC_SELF_CALIBRATION_INITIAL_PERIOD, &mut buf)
            .await?;

        Ok(u16::from_be_bytes([buf[0], buf[1]]))
    }

    #[cfg(feature = "scd41")]
    async fn set_automatic_self_calibration_standard_period(
        &mut self,
        hours: u16,
    ) -> Result<(), Error<E>> {
        self.write_command_with_data(SET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD, hours)
            .await?;
        Ok(())
    }

    #[cfg(feature = "scd41")]
    async fn get_automatic_self_calibration_standard_period(&mut self) -> Result<u16, Error<E>> {
        let mut buf = [0; 3];
        self.read_command(GET_AUTOMATIC_SELF_CALIBRATION_STANDARD_PERIOD, &mut buf)
            .await?;

        Ok(u16::from_be_bytes([buf[0], buf[1]]))
    }
}
