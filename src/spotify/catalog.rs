use super::*;

#[derive(Clone, Debug, Default)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artwork_url: Option<String>,
    pub artist_names: Vec<String>,
    pub artist_ids: Vec<String>,
    pub release_date: String,
    pub kind: String,
    pub total_tracks: usize,
}

impl Album {
    pub fn as_track(&self) -> Track {
        Track {
            name: self.name.clone(),
            album: self.name.clone(),
            album_id: self.id.clone(),
            artwork_url: self.artwork_url.clone(),
            artist: self.artist_names.join(", "),
            artist_names: self.artist_names.clone(),
            artist_ids: self.artist_ids.clone(),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_offset: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct ArtistOverview {
    pub artist: Artist,
    pub songs: SongPage,
    pub albums: Page<Album>,
    pub notice: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct AlbumDetails {
    pub album: Album,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, Default)]
pub struct SearchCatalog {
    pub tracks: Vec<Track>,
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub playlists: Vec<Playlist>,
}

#[derive(Clone, Debug)]
pub struct SongCursor {
    pub group: String,
    pub offset: usize,
}

#[derive(Clone, Debug, Default)]
pub struct SongPage {
    pub items: Vec<Track>,
    pub next: Option<SongCursor>,
}

#[derive(Deserialize)]
struct ArtistRecord {
    id: String,
    name: String,
    #[serde(default)]
    images: Vec<ApiImage>,
}

#[derive(Deserialize, Default)]
struct AlbumRecord {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    images: Vec<ApiImage>,
    #[serde(default)]
    artists: Vec<ApiArtist>,
    #[serde(default)]
    release_date: String,
    #[serde(default)]
    album_type: String,
    #[serde(default)]
    total_tracks: usize,
    tracks: Option<TrackPage>,
}

impl AlbumRecord {
    fn summary(&self) -> Album {
        Album {
            id: self.id.clone(),
            name: self.name.clone(),
            artwork_url: self.images.first().map(|image| image.url.clone()),
            artist_names: self
                .artists
                .iter()
                .map(|artist| artist.name.clone())
                .collect(),
            artist_ids: self
                .artists
                .iter()
                .map(|artist| artist.id.clone())
                .collect(),
            release_date: self.release_date.clone(),
            kind: self.album_type.clone(),
            total_tracks: self.total_tracks,
        }
    }
}

#[derive(Deserialize)]
struct AlbumPage {
    items: Vec<AlbumRecord>,
    next: Option<String>,
    #[serde(default)]
    offset: usize,
}

fn get<T: serde::de::DeserializeOwned>(
    client: &Client,
    token: &str,
    url: &str,
    params: &[(&str, String)],
) -> Result<T, String> {
    api_response(
        client.get(url).bearer_auth(token).query(params),
        "Could not load Spotify catalogue",
    )?
    .json()
    .map_err(|error| format!("Could not read Spotify catalogue: {error}"))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

pub fn artist_overview(token: &str, id: &str, name: &str) -> Result<ArtistOverview, String> {
    let client = http_client()?;
    let record: ArtistRecord = if valid_id(id) {
        get(&client, token, &format!("{API_BASE}/artists/{id}"), &[])?
    } else {
        #[derive(Deserialize)]
        struct Artists {
            items: Vec<ArtistRecord>,
        }
        #[derive(Deserialize)]
        struct Search {
            artists: Artists,
        }
        let results: Search = get(
            &client,
            token,
            &format!("{API_BASE}/search"),
            &[
                ("q", name.into()),
                ("type", "artist".into()),
                ("limit", "10".into()),
            ],
        )?;
        results
            .artists
            .items
            .into_iter()
            .find(|artist| artist.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| {
                "Couldn't identify this artist. Try opening a newer search result.".to_string()
            })?
    };
    let artist = Artist {
        id: record.id,
        name: record.name,
        image_url: record.images.first().map(|image| image.url.clone()),
    };
    let albums = artist_albums(token, &artist.id, 0);
    let songs = artist_songs(token, &artist.id, &artist.name, None);
    let notice = match (&albums, &songs) {
        (Err(error), _) => Some(error.clone()),
        (_, Err(error)) => Some(error.clone()),
        _ => None,
    };
    Ok(ArtistOverview {
        artist,
        albums: albums.unwrap_or_default(),
        songs: songs.unwrap_or_default(),
        notice,
    })
}

pub fn artist_albums(token: &str, id: &str, offset: usize) -> Result<Page<Album>, String> {
    artist_albums_from(&http_client()?, token, API_BASE, id, offset)
}

fn artist_albums_from(
    client: &Client,
    token: &str,
    base: &str,
    id: &str,
    offset: usize,
) -> Result<Page<Album>, String> {
    if !valid_id(id) {
        return Err("This artist has no Spotify catalogue ID.".into());
    }
    let page: AlbumPage = get(
        client,
        token,
        &format!("{base}/artists/{id}/albums"),
        &[
            ("include_groups", "album,single,compilation".into()),
            ("limit", "10".into()),
            ("offset", offset.to_string()),
        ],
    )?;
    let next_offset = page
        .next
        .filter(|_| !page.items.is_empty())
        .map(|_| page.offset + page.items.len());
    Ok(Page {
        items: page.items.iter().map(AlbumRecord::summary).collect(),
        next_offset,
    })
}

pub fn artist_songs(
    token: &str,
    id: &str,
    name: &str,
    cursor: Option<SongCursor>,
) -> Result<SongPage, String> {
    artist_songs_from(&http_client()?, token, API_BASE, id, name, cursor)
}

fn artist_songs_from(
    client: &Client,
    token: &str,
    base: &str,
    id: &str,
    name: &str,
    cursor: Option<SongCursor>,
) -> Result<SongPage, String> {
    if !valid_id(id) {
        return Err("This artist has no Spotify catalogue ID.".into());
    }
    let mut result = SongPage::default();
    if cursor.is_none() {
        let query = format!("artist:\"{}\"", name.replace(['"', '\\'], " "));
        let searched = get::<SearchResponse>(
            client,
            token,
            &format!("{base}/search"),
            &[
                ("q", query),
                ("type", "track".into()),
                ("limit", "10".into()),
            ],
        );
        match searched {
            Ok(response) => {
                result.items = response
                    .tracks
                    .into_iter()
                    .flat_map(|page| page.items)
                    .map(into_track)
                    .filter(|track| track.artist_ids.iter().any(|credit| credit == id))
                    .collect()
            }
            Err(error) if error.contains("429") => return Err(error),
            Err(_) => {}
        }
    }
    // Search can return only a handful of songs. Read the actual releases and
    // advance one album at a time rather than prefetching an entire discography.
    let mut cursor = cursor.unwrap_or(SongCursor {
        group: "album".into(),
        offset: 0,
    });
    loop {
        let page: AlbumPage = get(
            client,
            token,
            &format!("{base}/artists/{id}/albums"),
            &[
                ("include_groups", cursor.group.clone()),
                ("limit", "1".into()),
                ("offset", cursor.offset.to_string()),
            ],
        )?;
        let Some(record) = page.items.first() else {
            if cursor.group == "album" {
                cursor = SongCursor {
                    group: "single,compilation".into(),
                    offset: 0,
                };
                continue;
            }
            break;
        };
        if !valid_id(&record.id) {
            return Err("This release has no Spotify catalogue ID.".into());
        }
        let album = album_from(client, token, base, &record.id)?;
        for track in album
            .tracks
            .into_iter()
            .filter(|track| track.artist_ids.iter().any(|credit| credit == id))
        {
            if !result
                .items
                .iter()
                .any(|previous| previous.uri == track.uri)
            {
                result.items.push(track);
            }
        }
        result.next = if page.next.is_some() {
            Some(SongCursor {
                group: cursor.group,
                offset: page.offset + page.items.len(),
            })
        } else if cursor.group == "album" {
            Some(SongCursor {
                group: "single,compilation".into(),
                offset: 0,
            })
        } else {
            None
        };
        break;
    }
    Ok(result)
}

pub fn album_for_track(token: &str, seed: &Track) -> Result<AlbumDetails, String> {
    let client = http_client()?;
    let id = if valid_id(&seed.album_id) {
        seed.album_id.clone()
    } else {
        let track_id = seed.uri.strip_prefix("spotify:track:").unwrap_or(&seed.id);
        if !valid_id(track_id) {
            return Err("This song has no Spotify album.".into());
        }
        let track: ApiTrack = get(
            &client,
            token,
            &format!("{API_BASE}/tracks/{track_id}"),
            &[],
        )?;
        track
            .album
            .map(|album| album.id)
            .filter(|id| valid_id(id))
            .ok_or("This song has no Spotify album.")?
    };
    album_from(&client, token, API_BASE, &id)
}

pub fn album_details(token: &str, id: &str) -> Result<AlbumDetails, String> {
    if !valid_id(id) {
        return Err("Invalid Spotify album ID.".into());
    }
    let client = http_client()?;
    album_from(&client, token, API_BASE, id)
}

fn album_from(client: &Client, token: &str, base: &str, id: &str) -> Result<AlbumDetails, String> {
    let record: AlbumRecord = get(client, token, &format!("{base}/albums/{id}"), &[])?;
    let album = record.summary();
    let mut result = AlbumDetails {
        album,
        tracks: Vec::new(),
    };
    let mut page = record.tracks;
    while let Some(tracks) = page.take() {
        let next_offset = tracks.offset + tracks.items.len();
        let has_next = tracks.next.is_some() && !tracks.items.is_empty();
        for (index, item) in tracks.items.into_iter().enumerate() {
            let mut track = into_track(item);
            track.context_position = Some(tracks.offset + index);
            track.album = result.album.name.clone();
            track.album_id = result.album.id.clone();
            track.artwork_url = result.album.artwork_url.clone();
            result.tracks.push(track);
        }
        if has_next {
            page = Some(get(
                client,
                token,
                &format!("{base}/albums/{id}/tracks"),
                &[("limit", "10".into()), ("offset", next_offset.to_string())],
            )?);
            if page.as_ref().is_some_and(|page| page.offset < next_offset) {
                return Err("Spotify returned a repeated album page. Try again.".into());
            }
        }
    }
    Ok(result)
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
struct SearchItemsPage<T> {
    #[serde(default)]
    items: Vec<Option<T>>,
}

#[derive(Deserialize)]
struct CatalogSearchResponse {
    tracks: Option<SearchItemsPage<ApiTrack>>,
    artists: Option<SearchItemsPage<ArtistRecord>>,
    albums: Option<SearchItemsPage<AlbumRecord>>,
    playlists: Option<SearchItemsPage<ApiPlaylist>>,
}

pub fn search_catalog(token: &str, query: &str) -> Result<SearchCatalog, String> {
    let client = http_client()?;
    search_catalog_from(&client, token, API_BASE, query)
}

fn search_catalog_from(
    client: &Client,
    token: &str,
    base: &str,
    query: &str,
) -> Result<SearchCatalog, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(SearchCatalog::default());
    }
    let response: CatalogSearchResponse = get(
        client,
        token,
        &format!("{base}/search"),
        &[
            ("q", trimmed.into()),
            ("type", "track,artist,album,playlist".into()),
            ("limit", "10".into()),
        ],
    )?;
    let tracks = response
        .tracks
        .map(|page| {
            page.items
                .into_iter()
                .flatten()
                .filter(|t| t.is_playable != Some(false) && t.uri.starts_with("spotify:track:"))
                .map(into_track)
                .collect()
        })
        .unwrap_or_default();
    let artists = response
        .artists
        .map(|page| {
            page.items
                .into_iter()
                .flatten()
                .filter(|a| !a.name.trim().is_empty())
                .map(|a| Artist {
                    id: a.id,
                    name: a.name,
                    image_url: a.images.first().map(|img| img.url.clone()),
                })
                .collect()
        })
        .unwrap_or_default();
    let albums = response
        .albums
        .map(|page| {
            page.items
                .into_iter()
                .flatten()
                .filter(|a| !a.id.is_empty() && !a.name.trim().is_empty())
                .map(|a| a.summary())
                .collect()
        })
        .unwrap_or_default();
    let playlists = response
        .playlists
        .map(|page| {
            page.items
                .into_iter()
                .flatten()
                .filter(|p| !p.id.is_empty() && !p.name.trim().is_empty())
                .map(into_playlist)
                .collect()
        })
        .unwrap_or_default();
    Ok(SearchCatalog {
        tracks,
        artists,
        albums,
        playlists,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn server(
        responses: Vec<(&'static str, serde_json::Value)>,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            for (expected, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_callback_request(
                    &mut stream,
                    &AtomicBool::new(false),
                    Instant::now() + Duration::from_secs(5),
                )
                .unwrap();
                assert!(request.contains(expected), "unexpected catalogue request");
                callback_response(&mut stream, "200 OK", &body.to_string());
            }
        });
        (base, worker)
    }

    #[test]
    fn releases_have_more_pages_and_album_tracks_keep_ids_artwork_and_order() {
        let tracks = |start: usize, end: usize| {
            (start..end).map(|i| json!({"id":format!("t{i}"),"uri":format!("spotify:track:t{i}"),"name":format!("Track {i}"),"artists":[{"id":"artist1","name":"Artist"}]})).collect::<Vec<_>>()
        };
        let (base, worker) = server(vec![
            (
                "/artists/artist1/albums?",
                json!({"offset":0,"next":"https://api.spotify.com/v1/artists/artist1/albums?offset=10","items":[{"id":"album1","name":"Album","release_date":"2026","album_type":"album","artists":[{"id":"artist1","name":"Artist"}]}]}),
            ),
            (
                "/albums/album1 ",
                json!({"id":"album1","name":"Album","images":[{"url":"https://example.test/cover"}],"total_tracks":23,"tracks":{"offset":0,"next":"https://api.spotify.com/v1/albums/album1/tracks?offset=10","items":tracks(0,10)}}),
            ),
            (
                "offset=10",
                json!({"offset":10,"next":"https://api.spotify.com/v1/albums/album1/tracks?offset=20","items":tracks(10,20)}),
            ),
            (
                "offset=20",
                json!({"offset":20,"next":null,"items":tracks(20,23)}),
            ),
        ]);
        let client = http_client().unwrap();
        let releases = artist_albums_from(&client, "test-only", &base, "artist1", 0).unwrap();
        assert_eq!(releases.next_offset, Some(1));
        assert_eq!(releases.items[0].artist_ids, ["artist1"]);
        let details = album_from(&client, "test-only", &base, "album1").unwrap();
        assert_eq!(details.tracks.len(), 23);
        assert_eq!(details.tracks[22].id, "t22");
        assert!(details.tracks.iter().all(|track| track.album_id == "album1"
            && track.artist_ids == ["artist1"]
            && track.artwork_url.as_deref() == Some("https://example.test/cover")));
        worker.join().unwrap();
    }

    #[test]
    fn metadata_keeps_each_artist_id_aligned_and_old_bookmarks_still_load() {
        let item: ApiTrack = serde_json::from_value(json!({"id":"track1","artists":[{"name":"Unknown"},{"id":"guest1","name":"Guest"}],"album":{"id":"album1","name":"Album"}})).unwrap();
        let track = into_track(item);
        assert_eq!(track.artist_ids, ["", "guest1"]);
        assert_eq!(track.album_id, "album1");
        let old: Track =
            serde_json::from_str(r#"{"name":"Saved song","artist_names":["Artist"]}"#).unwrap();
        assert!(old.album_id.is_empty());
        assert!(old.artist_ids.is_empty());
        assert!(!valid_id("../wrong/path"));
    }

    #[test]
    fn five_search_results_expand_from_releases_and_keep_a_discography_cursor() {
        let tracks = |start: usize, end: usize| {
            (start..end).map(|i| json!({"id":format!("t{i}"),"uri":format!("spotify:track:t{i}"),"name":format!("Song {i}"),"artists":[{"id":"artist1","name":"Artist"}]})).collect::<Vec<_>>()
        };
        let (base, worker) = server(vec![
            (
                "/search?",
                json!({"tracks":{"items":tracks(0,5),"total":5,"next":null}}),
            ),
            (
                "include_groups=album",
                json!({"offset":0,"next":"https://api.spotify.com/v1/artists/artist1/albums?offset=1","items":[{"id":"album1"}]}),
            ),
            (
                "/albums/album1 ",
                json!({"id":"album1","name":"Album","total_tracks":18,"tracks":{"offset":0,"next":null,"items":tracks(0,18)}}),
            ),
        ]);
        let page = artist_songs_from(
            &http_client().unwrap(),
            "test-only",
            &base,
            "artist1",
            "Artist",
            None,
        )
        .unwrap();
        assert_eq!(page.items.len(), 18);
        let next = page.next.unwrap();
        assert_eq!(next.group, "album");
        assert_eq!(next.offset, 1);
        worker.join().unwrap();
    }

    #[test]
    #[ignore = "Reads the existing account credentials and verifies live catalogue pages; prints counts only"]
    fn live_catalogue_smoke() {
        let config = crate::AppConfig::load();
        let refresh = load_refresh_token().expect("saved Spotify session");
        let token = restore_session(&refresh, &config.client_id).expect("Spotify session refresh");
        if let Some(refresh) = &token.refresh_token {
            save_refresh_token(refresh).unwrap();
        }
        for name in ["Juice WRLD", "TV Girl"] {
            let overview = artist_overview(&token.access_token, "", name).unwrap();
            assert!(overview.notice.is_none(), "{:?}", overview.notice);
            assert!(
                overview.songs.items.len() > 5,
                "artist songs remain unexpectedly short"
            );
            assert!(!overview.albums.items.is_empty());
            if name == "Juice WRLD" {
                let mut old = overview.songs.items[0].clone();
                let expected = old.album_id.clone();
                old.album_id.clear();
                let resolved = album_for_track(&token.access_token, &old).unwrap();
                assert_eq!(
                    resolved.album.id, expected,
                    "older playback metadata must resolve the exact album"
                );
            }
            let album = &overview.albums.items[0];
            let details = album_for_track(&token.access_token, &album.as_track()).unwrap();
            assert_eq!(details.tracks.len(), details.album.total_tracks);
            println!(
                "{name}: {} songs, more songs {}, {} releases, more releases {}, album {} tracks",
                overview.songs.items.len(),
                overview.songs.next.is_some(),
                overview.albums.items.len(),
                overview.albums.next_offset.is_some(),
                details.tracks.len()
            );
        }
    }

    #[test]
    fn multi_type_search_catalog_parses_tracks_artists_albums_and_playlists() {
        let (base, worker) = server(vec![(
            "type=track%2Cartist%2Calbum%2Cplaylist",
            json!({
                "tracks": {"items": [
                    {"id":"t1","uri":"spotify:track:t1","name":"Night Drive","duration_ms":180000,"explicit":true,"artists":[{"id":"a1","name":"Neon Pulse"}],"album":{"id":"al1","name":"After Hours"}},
                    null,
                    {"id":"t2","uri":"spotify:track:t2","name":"Blocked","is_playable":false,"artists":[{"id":"a1","name":"Neon Pulse"}]}
                ]},
                "artists": {"items": [
                    {"id":"a1","name":"Neon Pulse","images":[{"url":"https://example.test/a1.jpg"}]},
                    null
                ]},
                "albums": {"items": [
                    {"id":"al1","name":"After Hours","album_type":"album","release_date":"2024-01-01","total_tracks":10,"artists":[{"id":"a1","name":"Neon Pulse"}]}
                ]},
                "playlists": {"items": [
                    null,
                    {"id":"p1","name":"Synthwave Essentials","owner":{"id":"sp","display_name":"Editorial"},"tracks":{"total":42}}
                ]}
            }),
        )]);
        let catalog =
            search_catalog_from(&http_client().unwrap(), "test-only", &base, "neon").unwrap();
        assert_eq!(catalog.tracks.len(), 1);
        assert_eq!(catalog.tracks[0].name, "Night Drive");
        assert!(catalog.tracks[0].explicit);
        assert_eq!(catalog.artists.len(), 1);
        assert_eq!(catalog.artists[0].name, "Neon Pulse");
        assert_eq!(catalog.albums.len(), 1);
        assert_eq!(catalog.albums[0].name, "After Hours");
        assert_eq!(catalog.playlists.len(), 1);
        assert_eq!(catalog.playlists[0].name, "Synthwave Essentials");
        assert_eq!(catalog.playlists[0].total_items, 42);
        worker.join().unwrap();
    }
}
