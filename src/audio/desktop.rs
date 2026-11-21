use super::{Frame, Mixer};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::Receiver;

pub struct Handle {
    _stream: cpal::Stream,
}

pub fn open(receiver: Receiver<Frame>) -> Result<Handle, Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("没有可用的音频输出设备")?;
    let supported = device.default_output_config()?;
    let config: cpal::StreamConfig = supported.clone().into();
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&device, &config, receiver)?,
        cpal::SampleFormat::I16 => build::<i16>(&device, &config, receiver)?,
        cpal::SampleFormat::U16 => build::<u16>(&device, &config, receiver)?,
        format => return Err(format!("音频格式暂不可用：{format}").into()),
    };
    stream.play()?;
    Ok(Handle { _stream: stream })
}
fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    receiver: Receiver<Frame>,
) -> Result<cpal::Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let mut mixer = Mixer::new(config.sample_rate.0);
    mixer.reseed(super::super::entropy());
    let channels = config.channels as usize;
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            while let Ok(frame) = receiver.try_recv() {
                mixer.update(frame);
            }
            for frame in data.chunks_exact_mut(channels) {
                let stereo = mixer.sample();
                for (i, sample) in frame.iter_mut().enumerate() {
                    let value = if channels == 1 || i > 1 {
                        (stereo[0] + stereo[1]) * 0.5
                    } else {
                        stereo[i]
                    };
                    *sample = T::from_sample(value);
                }
            }
        },
        |error| eprintln!("音频输出错误：{error}"),
        None,
    )
}
