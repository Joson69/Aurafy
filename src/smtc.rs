//! Windows System Media Transport Controls (media keys and volume flyout).
#![cfg(target_os = "windows")]

use crate::{AppEvent, desktop, spotify::Track};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    sync::mpsc::Sender,
    time::{Duration, Instant},
};
use windows::{
    Foundation::{TimeSpan, TypedEventHandler, Uri},
    Media::{
        MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
        SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsTimelineProperties,
    },
    Storage::Streams::RandomAccessStreamReference,
    Win32::{
        Foundation::HWND,
        System::WinRT::{
            ISystemMediaTransportControlsInterop, RO_INIT_SINGLETHREADED, RoInitialize,
        },
    },
    core::{HSTRING, factory},
};

pub(crate) struct Controls {
    smtc: SystemMediaTransportControls,
    token: i64,
    last_playing: Option<bool>,
    last_has_track: Option<bool>,
    last_identity: String,
    last_art: String,
    last_timeline_at: Option<Instant>,
}

impl Controls {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        tx: Sender<AppEvent>,
    ) -> Result<Self, String> {
        let hwnd = match cc.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as _),
            _ => return Err("Missing Win32 window handle for SMTC".into()),
        };
        // eframe initializes COM/WinRT on the UI thread; RPC_E_CHANGED_MODE is fine.
        unsafe {
            let _ = RoInitialize(RO_INIT_SINGLETHREADED);
        }
        let interop: ISystemMediaTransportControlsInterop =
            factory::<SystemMediaTransportControls, ISystemMediaTransportControlsInterop>()
                .map_err(|e| e.to_string())?;
        let smtc: SystemMediaTransportControls =
            unsafe { interop.GetForWindow(hwnd) }.map_err(|e| e.to_string())?;
        smtc.SetIsEnabled(true).map_err(|e| e.to_string())?;
        smtc.SetIsPlayEnabled(true).map_err(|e| e.to_string())?;
        smtc.SetIsPauseEnabled(true).map_err(|e| e.to_string())?;
        smtc.SetIsNextEnabled(true).map_err(|e| e.to_string())?;
        smtc.SetIsPreviousEnabled(true).map_err(|e| e.to_string())?;
        let ctx = cc.egui_ctx.clone();
        let handler = TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            let Some(args) = args.as_ref() else {
                return Ok(());
            };
            let action = match args.Button()? {
                SystemMediaTransportControlsButton::Play => Some(desktop::Action::Play),
                SystemMediaTransportControlsButton::Pause
                | SystemMediaTransportControlsButton::Stop => Some(desktop::Action::Pause),
                SystemMediaTransportControlsButton::Next => Some(desktop::Action::Next),
                SystemMediaTransportControlsButton::Previous => Some(desktop::Action::Previous),
                _ => None,
            };
            if let Some(action) = action {
                let _ = tx.send(AppEvent::Desktop(action));
                ctx.request_repaint();
            }
            Ok(())
        });
        let token = smtc.ButtonPressed(&handler).map_err(|e| e.to_string())?;
        Ok(Self {
            smtc,
            token,
            last_playing: None,
            last_has_track: None,
            last_identity: String::new(),
            last_art: String::new(),
            last_timeline_at: None,
        })
    }

    pub(crate) fn sync(
        &mut self,
        track: &Track,
        is_playing: bool,
        position_ms: u64,
        duration_ms: u64,
    ) {
        let has_track = !track.uri.is_empty() || !track.name.is_empty();
        if self.last_playing != Some(is_playing) || self.last_has_track != Some(has_track) {
            let status = if !has_track {
                MediaPlaybackStatus::Stopped
            } else if is_playing {
                MediaPlaybackStatus::Playing
            } else {
                MediaPlaybackStatus::Paused
            };
            let _ = self.smtc.SetPlaybackStatus(status);
            self.last_playing = Some(is_playing);
            self.last_has_track = Some(has_track);
        }

        let identity = format!(
            "{}\0{}\0{}\0{}",
            track.uri, track.name, track.artist, track.album
        );
        let art = track.artwork_url.clone().unwrap_or_default();
        if identity != self.last_identity || art != self.last_art {
            self.last_identity = identity;
            self.last_art = art.clone();
            if let Ok(updater) = self.smtc.DisplayUpdater() {
                let _ = updater.SetType(MediaPlaybackType::Music);
                if let Ok(music) = updater.MusicProperties() {
                    let _ = music.SetTitle(&HSTRING::from(track.name.as_str()));
                    let _ = music.SetArtist(&HSTRING::from(track.artist.as_str()));
                    let _ = music.SetAlbumTitle(&HSTRING::from(track.album.as_str()));
                }
                if art.starts_with("https://")
                    && let Ok(uri) = Uri::CreateUri(&HSTRING::from(art.as_str()))
                    && let Ok(thumb) = RandomAccessStreamReference::CreateFromUri(&uri)
                {
                    let _ = updater.SetThumbnail(&thumb);
                } else {
                    let _ = updater.SetThumbnail(None);
                }
                let _ = updater.Update();
            }
            self.last_timeline_at = None;
        }

        let due = self
            .last_timeline_at
            .is_none_or(|t| is_playing && t.elapsed() >= Duration::from_secs(2));
        if due
            && duration_ms > 0
            && let Ok(timeline) = SystemMediaTransportControlsTimelineProperties::new()
        {
            let _ = timeline.SetStartTime(TimeSpan { Duration: 0 });
            let _ = timeline.SetEndTime(TimeSpan {
                Duration: (duration_ms.min(i64::MAX as u64 / 10_000) as i64) * 10_000,
            });
            let _ = timeline.SetPosition(TimeSpan {
                Duration: (position_ms.min(duration_ms).min(i64::MAX as u64 / 10_000) as i64)
                    * 10_000,
            });
            let _ = self.smtc.UpdateTimelineProperties(&timeline);
            self.last_timeline_at = Some(Instant::now());
        }
    }
}

impl Drop for Controls {
    fn drop(&mut self) {
        let _ = self.smtc.RemoveButtonPressed(self.token);
    }
}
