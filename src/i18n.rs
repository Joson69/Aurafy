//! Interface localization for English, Spanish, French, German, Japanese, and Korean.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    English,
    Spanish,
    French,
    German,
    Japanese,
    Korean,
}

impl Language {
    pub const ALL: [Self; 6] = [
        Self::English,
        Self::Spanish,
        Self::French,
        Self::German,
        Self::Japanese,
        Self::Korean,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Japanese => "日本語",
            Self::Korean => "한국어",
        }
    }
}

#[derive(Clone, Copy)]
pub enum Text {
    Home,
    Search,
    Dj,
    YourLibrary,
    Collection,
    LikedSongs,
    RecentlyPlayed,
    CreatePlaylist,
    Profile,
    Settings,
    SettingsSubtitle,
    SleepTimer,
    Account,
    LogOut,
    QuitAurafy,
    SpotifyConnected,
    SpotifyAccount,
    KeepRunningOnClose,
    KeepRunningOnCloseDesc,
    ResumeOnOpen,
    ResumeOnOpenDesc,
    SearchPlaceholder,
    TabGeneral,
    TabListening,
    TabAppearance,
    TabPrivacy,
    TabAccount,
    LanguageLabel,
    StartupAndBackground,
    OpenAurafyWith,
    CustomTitleBar,
    CustomTitleBarDesc,
    TrayIcon,
    KeyboardShortcuts,
    SoundAndStorage,
    Display,
    Theme,
    InterfaceZoom,
    Reset,
    Queue,
    UpNext,
    Lyrics,
    FollowSong,
    CloseLyrics,
    ConnectToDevice,
    RefreshDevices,
    AudioOutput,
    SystemDefault,
    Play,
    Pause,
    Previous,
    Next,
    Shuffle,
    Repeat,
    Mute,
    Unmute,
    Fullscreen,
    LikedSubtitle,
    RecentSubtitle,
    HomePlaylists,
    YourTopMusic,
    Songs,
    Artists,
    SearchResults,
    LibrarySubtitle,
    AurafyDj,
    NewChat,
    Send,
    MessageDjPlaceholder,
    YourSet,
    Updates,
    CheckForUpdates,
}

pub fn tr(lang: Language, key: Text) -> &'static str {
    use Language::*;
    use Text::*;
    match (lang, key) {
        (English, Home) => "Home",
        (Spanish, Home) => "Inicio",
        (French, Home) => "Accueil",
        (German, Home) => "Start",
        (Japanese, Home) => "ホーム",
        (Korean, Home) => "홈",

        (English, Search) => "Search",
        (Spanish, Search) => "Buscar",
        (French, Search) => "Rechercher",
        (German, Search) => "Suche",
        (Japanese, Search) => "検索",
        (Korean, Search) => "검색",

        (English | Spanish | French | German | Japanese | Korean, Dj) => "DJ",

        (English, YourLibrary) => "Your library",
        (Spanish, YourLibrary) => "Tu biblioteca",
        (French, YourLibrary) => "Bibliothèque",
        (German, YourLibrary) => "Bibliothek",
        (Japanese, YourLibrary) => "マイライブラリ",
        (Korean, YourLibrary) => "내 라이브러리",

        (English, Collection) => "COLLECTION",
        (Spanish, Collection) => "COLECCIÓN",
        (French, Collection) => "COLLECTION",
        (German, Collection) => "SAMMLUNG",
        (Japanese, Collection) => "コレクション",
        (Korean, Collection) => "컬렉션",

        (English, LikedSongs) => "Liked Songs",
        (Spanish, LikedSongs) => "Canciones que te gustan",
        (French, LikedSongs) => "Titres likés",
        (German, LikedSongs) => "Lieblingssongs",
        (Japanese, LikedSongs) => "お気に入りの曲",
        (Korean, LikedSongs) => "좋아요 표시한 곡",

        (English, RecentlyPlayed) => "Recently played",
        (Spanish, RecentlyPlayed) => "Escuchado recientemente",
        (French, RecentlyPlayed) => "Écoutés récemment",
        (German, RecentlyPlayed) => "Zuletzt gehört",
        (Japanese, RecentlyPlayed) => "最近再生した曲",
        (Korean, RecentlyPlayed) => "최근 재생한 곡",

        (English, CreatePlaylist) => "Create playlist",
        (Spanish, CreatePlaylist) => "Crear playlist",
        (French, CreatePlaylist) => "Créer une playlist",
        (German, CreatePlaylist) => "Playlist erstellen",
        (Japanese, CreatePlaylist) => "プレイリストを作成",
        (Korean, CreatePlaylist) => "플레이리스트 만들기",

        (English, Profile) => "Profile",
        (Spanish, Profile) => "Perfil",
        (French, Profile) => "Profil",
        (German, Profile) => "Profil",
        (Japanese, Profile) => "プロフィール",
        (Korean, Profile) => "프로필",

        (English, Settings) => "Settings",
        (Spanish, Settings) => "Ajustes",
        (French, Settings) => "Paramètres",
        (German, Settings) => "Einstellungen",
        (Japanese, Settings) => "設定",
        (Korean, Settings) => "설정",

        (English, SettingsSubtitle) => "Playback, appearance, privacy and account.",
        (Spanish, SettingsSubtitle) => "Reproducción, apariencia, privacidad y cuenta.",
        (French, SettingsSubtitle) => "Lecture, apparence, confidentialité et compte.",
        (German, SettingsSubtitle) => "Wiedergabe, Darstellung, Datenschutz und Konto.",
        (Japanese, SettingsSubtitle) => "再生、外観、プライバシー、アカウント。",
        (Korean, SettingsSubtitle) => "재생, 화면, 개인정보 및 계정.",

        (English, SleepTimer) => "Sleep timer",
        (Spanish, SleepTimer) => "Temporizador",
        (French, SleepTimer) => "Minuteur de veille",
        (German, SleepTimer) => "Sleep-Timer",
        (Japanese, SleepTimer) => "スリープタイマー",
        (Korean, SleepTimer) => "취침 타이머",

        (English, Account) => "Account",
        (Spanish, Account) => "Cuenta",
        (French, Account) => "Compte",
        (German, Account) => "Konto",
        (Japanese, Account) => "アカウント",
        (Korean, Account) => "계정",

        (English, LogOut) => "Log out",
        (Spanish, LogOut) => "Cerrar sesión",
        (French, LogOut) => "Déconnexion",
        (German, LogOut) => "Abmelden",
        (Japanese, LogOut) => "ログアウト",
        (Korean, LogOut) => "로그아웃",

        (English, QuitAurafy) => "Quit Aurafy",
        (Spanish, QuitAurafy) => "Salir de Aurafy",
        (French, QuitAurafy) => "Quitter Aurafy",
        (German, QuitAurafy) => "Aurafy beenden",
        (Japanese, QuitAurafy) => "Aurafy を終了",
        (Korean, QuitAurafy) => "Aurafy 종료",

        (English, SpotifyConnected) => "Spotify connected",
        (Spanish, SpotifyConnected) => "Spotify conectado",
        (French, SpotifyConnected) => "Spotify connecté",
        (German, SpotifyConnected) => "Spotify verbunden",
        (Japanese, SpotifyConnected) => "Spotify 接続済み",
        (Korean, SpotifyConnected) => "Spotify 연결됨",

        (English, SpotifyAccount) => "Spotify account",
        (Spanish, SpotifyAccount) => "Cuenta de Spotify",
        (French, SpotifyAccount) => "Compte Spotify",
        (German, SpotifyAccount) => "Spotify-Konto",
        (Japanese, SpotifyAccount) => "Spotify アカウント",
        (Korean, SpotifyAccount) => "Spotify 계정",

        (English, KeepRunningOnClose) => "Keep running when I close Aurafy",
        (Spanish, KeepRunningOnClose) => "Seguir ejecutándose al cerrar Aurafy",
        (French, KeepRunningOnClose) => "Rester actif à la fermeture d'Aurafy",
        (German, KeepRunningOnClose) => "Beim Schließen im Hintergrund weiterlaufen",
        (Japanese, KeepRunningOnClose) => "ウィンドウを閉じてもバックグラウンドで実行",
        (Korean, KeepRunningOnClose) => "창을 닫아도 백그라운드에서 계속 실행",

        (English, KeepRunningOnCloseDesc) => {
            "Music and the sleep timer keep working in the tray. Use Quit to exit."
        }
        (Spanish, KeepRunningOnCloseDesc) => {
            "La música y el temporizador siguen activos en la bandeja. Usa Salir para cerrar."
        }
        (French, KeepRunningOnCloseDesc) => {
            "La musique et le minuteur continuent dans la zone de notification."
        }
        (German, KeepRunningOnCloseDesc) => {
            "Musik und Sleep-Timer laufen im Infobereich weiter. Zum Beenden Quit wählen."
        }
        (Japanese, KeepRunningOnCloseDesc) => {
            "トレイで再生とスリープタイマーを継続します。終了するには「終了」を使います。"
        }
        (Korean, KeepRunningOnCloseDesc) => {
            "트레이에서 음악과 취침 타이머가 계속 작동합니다. 종료하려면 종료를 누르세요."
        }

        (English, ResumeOnOpen) => "Resume music when Aurafy opens",
        (Spanish, ResumeOnOpen) => "Reanudar música al abrir Aurafy",
        (French, ResumeOnOpen) => "Reprendre la musique à l'ouverture",
        (German, ResumeOnOpen) => "Musik beim Öffnen fortsetzen",
        (Japanese, ResumeOnOpen) => "起動時に再生を再開する",
        (Korean, ResumeOnOpen) => "Aurafy 시작 시 음악 다시 재생",

        (English, ResumeOnOpenDesc) => "Continue your song and position automatically.",
        (Spanish, ResumeOnOpenDesc) => "Continúa tu canción y posición automáticamente.",
        (French, ResumeOnOpenDesc) => "Reprend automatiquement le titre et la position.",
        (German, ResumeOnOpenDesc) => "Setzt Song und Position automatisch fort.",
        (Japanese, ResumeOnOpenDesc) => "前回の曲と再生位置を自動で続けます。",
        (Korean, ResumeOnOpenDesc) => "이전 곡과 재생 위치를 자동으로 이어서 재생합니다.",

        (English, SearchPlaceholder) => "Search songs, artists, albums",
        (Spanish, SearchPlaceholder) => "Buscar canciones, artistas, álbumes",
        (French, SearchPlaceholder) => "Rechercher titres, artistes, albums",
        (German, SearchPlaceholder) => "Songs, Künstler, Alben suchen",
        (Japanese, SearchPlaceholder) => "曲、アーティスト、アルバムを検索",
        (Korean, SearchPlaceholder) => "곡, 아티스트, 앨범 검색",

        (English, TabGeneral) => "General",
        (Spanish, TabGeneral) => "General",
        (French, TabGeneral) => "Général",
        (German, TabGeneral) => "Allgemein",
        (Japanese, TabGeneral) => "一般",
        (Korean, TabGeneral) => "일반",

        (English, TabListening) => "Listening & sound",
        (Spanish, TabListening) => "Audio y reproducción",
        (French, TabListening) => "Écoute et son",
        (German, TabListening) => "Wiedergabe & Ton",
        (Japanese, TabListening) => "再生とサウンド",
        (Korean, TabListening) => "재생 및 사운드",

        (English, TabAppearance) => "Appearance",
        (Spanish, TabAppearance) => "Apariencia",
        (French, TabAppearance) => "Apparence",
        (German, TabAppearance) => "Darstellung",
        (Japanese, TabAppearance) => "外観",
        (Korean, TabAppearance) => "화면 스타일",

        (English, TabPrivacy) => "Privacy",
        (Spanish, TabPrivacy) => "Privacidad",
        (French, TabPrivacy) => "Confidentialité",
        (German, TabPrivacy) => "Datenschutz",
        (Japanese, TabPrivacy) => "プライバシー",
        (Korean, TabPrivacy) => "개인정보",

        (English, TabAccount) => "Account",
        (Spanish, TabAccount) => "Cuenta",
        (French, TabAccount) => "Compte",
        (German, TabAccount) => "Konto",
        (Japanese, TabAccount) => "アカウント",
        (Korean, TabAccount) => "계정",

        (English, LanguageLabel) => "Language",
        (Spanish, LanguageLabel) => "Idioma",
        (French, LanguageLabel) => "Langue",
        (German, LanguageLabel) => "Sprache",
        (Japanese, LanguageLabel) => "言語",
        (Korean, LanguageLabel) => "언어",

        (English, StartupAndBackground) => "Startup & background",
        (Spanish, StartupAndBackground) => "Inicio y segundo plano",
        (French, StartupAndBackground) => "Démarrage et arrière-plan",
        (German, StartupAndBackground) => "Start & Hintergrund",
        (Japanese, StartupAndBackground) => "起動とバックグラウンド",
        (Korean, StartupAndBackground) => "시작 및 백그라운드",

        (English, OpenAurafyWith) => "Open Aurafy with",
        (Spanish, OpenAurafyWith) => "Abrir Aurafy en",
        (French, OpenAurafyWith) => "Ouvrir Aurafy sur",
        (German, OpenAurafyWith) => "Aurafy öffnen mit",
        (Japanese, OpenAurafyWith) => "起動時の画面",
        (Korean, OpenAurafyWith) => "시작 페이지",

        (English, CustomTitleBar) => "Custom title bar",
        (Spanish, CustomTitleBar) => "Barra de título personalizada",
        (French, CustomTitleBar) => "Barre de titre personnalisée",
        (German, CustomTitleBar) => "Eigene Titelleiste",
        (Japanese, CustomTitleBar) => "カスタムタイトルバー",
        (Korean, CustomTitleBar) => "사용자 지정 제목 표시줄",

        (English, CustomTitleBarDesc) => {
            "Use Aurafy's window controls instead of the Windows title bar."
        }
        (Spanish, CustomTitleBarDesc) => {
            "Usa los controles de Aurafy en lugar de la barra de Windows."
        }
        (French, CustomTitleBarDesc) => {
            "Utiliser les contrôles d'Aurafy au lieu de la barre Windows."
        }
        (German, CustomTitleBarDesc) => "Aurafy-Steuerung statt Windows-Titelleiste verwenden.",
        (Japanese, CustomTitleBarDesc) => {
            "Windows 標準バーの代わりに Aurafy の操作バーを使用します。"
        }
        (Korean, CustomTitleBarDesc) => "Windows 제목 표시줄 대신 Aurafy 창 컨트롤을 사용합니다.",

        (English, TrayIcon) => "Tray icon",
        (Spanish, TrayIcon) => "Icono de bandeja",
        (French, TrayIcon) => "Icône de barre",
        (German, TrayIcon) => "Infobereich-Symbol",
        (Japanese, TrayIcon) => "トレイアイコン",
        (Korean, TrayIcon) => "트레이 아이콘",

        (English, KeyboardShortcuts) => "Keyboard shortcuts",
        (Spanish, KeyboardShortcuts) => "Atajos de teclado",
        (French, KeyboardShortcuts) => "Raccourcis clavier",
        (German, KeyboardShortcuts) => "Tastenkürzel",
        (Japanese, KeyboardShortcuts) => "キーボードショートカット",
        (Korean, KeyboardShortcuts) => "키보드 단축키",

        (English, SoundAndStorage) => "Sound & storage",
        (Spanish, SoundAndStorage) => "Sonido y almacenamiento",
        (French, SoundAndStorage) => "Son et stockage",
        (German, SoundAndStorage) => "Ton & Speicher",
        (Japanese, SoundAndStorage) => "サウンドとストレージ",
        (Korean, SoundAndStorage) => "사운드 및 저장소",

        (English, Display) => "Display",
        (Spanish, Display) => "Pantalla",
        (French, Display) => "Affichage",
        (German, Display) => "Anzeige",
        (Japanese, Display) => "表示",
        (Korean, Display) => "화면",

        (English, Theme) => "Theme",
        (Spanish, Theme) => "Tema",
        (French, Theme) => "Thème",
        (German, Theme) => "Design",
        (Japanese, Theme) => "テーマ",
        (Korean, Theme) => "테마",

        (English, InterfaceZoom) => "Interface zoom",
        (Spanish, InterfaceZoom) => "Escala de interfaz",
        (French, InterfaceZoom) => "Zoom de l'interface",
        (German, InterfaceZoom) => "Oberflächen-Zoom",
        (Japanese, InterfaceZoom) => "表示倍率",
        (Korean, InterfaceZoom) => "화면 배율",

        (English, Reset) => "Reset",
        (Spanish, Reset) => "Restablecer",
        (French, Reset) => "Réinitialiser",
        (German, Reset) => "Zurücksetzen",
        (Japanese, Reset) => "リセット",
        (Korean, Reset) => "초기화",

        (English, Queue) => "Queue",
        (Spanish, Queue) => "Cola",
        (French, Queue) => "File d'attente",
        (German, Queue) => "Warteschlange",
        (Japanese, Queue) => "再生キュー",
        (Korean, Queue) => "재생 대기열",

        (English, UpNext) => "Up next",
        (Spanish, UpNext) => "A continuación",
        (French, UpNext) => "À suivre",
        (German, UpNext) => "Als Nächstes",
        (Japanese, UpNext) => "次に再生",
        (Korean, UpNext) => "다음 곡",

        (English, Lyrics) => "Lyrics",
        (Spanish, Lyrics) => "Letras",
        (French, Lyrics) => "Paroles",
        (German, Lyrics) => "Songtexte",
        (Japanese, Lyrics) => "歌詞",
        (Korean, Lyrics) => "가사",

        (English, FollowSong) => "Follow song",
        (Spanish, FollowSong) => "Seguir canción",
        (French, FollowSong) => "Suivre le titre",
        (German, FollowSong) => "Song folgen",
        (Japanese, FollowSong) => "曲に合わせる",
        (Korean, FollowSong) => "곡 따라가기",

        (English, CloseLyrics) => "Close lyrics",
        (Spanish, CloseLyrics) => "Cerrar letras",
        (French, CloseLyrics) => "Fermer les paroles",
        (German, CloseLyrics) => "Songtext schließen",
        (Japanese, CloseLyrics) => "歌詞を閉じる",
        (Korean, CloseLyrics) => "가사 닫기",

        (English, ConnectToDevice) => "Connect to a device",
        (Spanish, ConnectToDevice) => "Conectar a un dispositivo",
        (French, ConnectToDevice) => "Se connecter à un appareil",
        (German, ConnectToDevice) => "Mit Gerät verbinden",
        (Japanese, ConnectToDevice) => "デバイスに接続",
        (Korean, ConnectToDevice) => "기기에 연결",

        (English, RefreshDevices) => "Refresh devices",
        (Spanish, RefreshDevices) => "Actualizar dispositivos",
        (French, RefreshDevices) => "Actualiser les appareils",
        (German, RefreshDevices) => "Geräte aktualisieren",
        (Japanese, RefreshDevices) => "デバイスを更新",
        (Korean, RefreshDevices) => "기기 새로고침",

        (English, AudioOutput) => "Audio output",
        (Spanish, AudioOutput) => "Salida de audio",
        (French, AudioOutput) => "Sortie audio",
        (German, AudioOutput) => "Audioausgabe",
        (Japanese, AudioOutput) => "オーディオ出力",
        (Korean, AudioOutput) => "오디오 출력",

        (English, SystemDefault) => "System default",
        (Spanish, SystemDefault) => "Predeterminado del sistema",
        (French, SystemDefault) => "Système par défaut",
        (German, SystemDefault) => "Systemstandard",
        (Japanese, SystemDefault) => "システム既定",
        (Korean, SystemDefault) => "시스템 기본값",

        (English, Play) => "Play",
        (Spanish, Play) => "Reproducir",
        (French, Play) => "Lecture",
        (German, Play) => "Wiedergabe",
        (Japanese, Play) => "再生",
        (Korean, Play) => "재생",

        (English, Pause) => "Pause",
        (Spanish, Pause) => "Pausar",
        (French, Pause) => "Pause",
        (German, Pause) => "Pause",
        (Japanese, Pause) => "一時停止",
        (Korean, Pause) => "일시정지",

        (English, Previous) => "Previous",
        (Spanish, Previous) => "Anterior",
        (French, Previous) => "Précédent",
        (German, Previous) => "Zurück",
        (Japanese, Previous) => "前の曲",
        (Korean, Previous) => "이전 곡",

        (English, Next) => "Next",
        (Spanish, Next) => "Siguiente",
        (French, Next) => "Suivant",
        (German, Next) => "Weiter",
        (Japanese, Next) => "次の曲",
        (Korean, Next) => "다음 곡",

        (English, Shuffle) => "Shuffle",
        (Spanish, Shuffle) => "Aleatorio",
        (French, Shuffle) => "Aléatoire",
        (German, Shuffle) => "Zufallswiedergabe",
        (Japanese, Shuffle) => "シャッフル",
        (Korean, Shuffle) => "셔플",

        (English, Repeat) => "Repeat",
        (Spanish, Repeat) => "Repetir",
        (French, Repeat) => "Répéter",
        (German, Repeat) => "Wiederholen",
        (Japanese, Repeat) => "リピート",
        (Korean, Repeat) => "반복",

        (English, Mute) => "Mute",
        (Spanish, Mute) => "Silenciar",
        (French, Mute) => "Muet",
        (German, Mute) => "Stumm",
        (Japanese, Mute) => "ミュート",
        (Korean, Mute) => "음소거",

        (English, Unmute) => "Unmute",
        (Spanish, Unmute) => "Activar sonido",
        (French, Unmute) => "Rétablir le son",
        (German, Unmute) => "Ton an",
        (Japanese, Unmute) => "ミュート解除",
        (Korean, Unmute) => "음소거 해제",

        (English, Fullscreen) => "Fullscreen",
        (Spanish, Fullscreen) => "Pantalla completa",
        (French, Fullscreen) => "Plein écran",
        (German, Fullscreen) => "Vollbild",
        (Japanese, Fullscreen) => "全画面",
        (Korean, Fullscreen) => "전체 화면",

        (English, LikedSubtitle) => "All your favorites, one play away.",
        (Spanish, LikedSubtitle) => "Todos tus favoritos, a un clic.",
        (French, LikedSubtitle) => "Tous vos favoris, à portée de lecture.",
        (German, LikedSubtitle) => "Alle Favoriten einen Klick entfernt.",
        (Japanese, LikedSubtitle) => "お気に入りの曲をすぐに再生。",
        (Korean, LikedSubtitle) => "좋아하는 곡을 한 번에 재생하세요.",

        (English, RecentSubtitle) => "Back to the songs you've been spending time with.",
        (Spanish, RecentSubtitle) => "Vuelve a las canciones que has estado escuchando.",
        (French, RecentSubtitle) => "Retrouvez les titres écoutés récemment.",
        (German, RecentSubtitle) => "Zurück zu deinen zuletzt gespielten Songs.",
        (Japanese, RecentSubtitle) => "最近よく聴いている曲に戻りましょう。",
        (Korean, RecentSubtitle) => "최근에 자주 들었던 곡으로 돌아가세요.",

        (English, HomePlaylists) => "Playlists",
        (Spanish, HomePlaylists) => "Playlists",
        (French, HomePlaylists) => "Playlists",
        (German, HomePlaylists) => "Playlists",
        (Japanese, HomePlaylists) => "プレイリスト",
        (Korean, HomePlaylists) => "플레이리스트",

        (English, YourTopMusic) => "Your top music",
        (Spanish, YourTopMusic) => "Tu música principal",
        (French, YourTopMusic) => "Vos titres préférés",
        (German, YourTopMusic) => "Deine Top-Musik",
        (Japanese, YourTopMusic) => "トップミュージック",
        (Korean, YourTopMusic) => "인기 음악",

        (English, Songs) => "Songs",
        (Spanish, Songs) => "Canciones",
        (French, Songs) => "Titres",
        (German, Songs) => "Songs",
        (Japanese, Songs) => "曲",
        (Korean, Songs) => "곡",

        (English, Artists) => "Artists",
        (Spanish, Artists) => "Artistas",
        (French, Artists) => "Artistes",
        (German, Artists) => "Künstler",
        (Japanese, Artists) => "アーティスト",
        (Korean, Artists) => "아티스트",

        (English, SearchResults) => "Search results",
        (Spanish, SearchResults) => "Resultados de búsqueda",
        (French, SearchResults) => "Résultats de recherche",
        (German, SearchResults) => "Suchergebnisse",
        (Japanese, SearchResults) => "検索結果",
        (Korean, SearchResults) => "검색 결과",

        (English, LibrarySubtitle) => "Playlists and saved music.",
        (Spanish, LibrarySubtitle) => "Playlists y música guardada.",
        (French, LibrarySubtitle) => "Playlists et musique enregistrée.",
        (German, LibrarySubtitle) => "Playlists und gespeicherte Musik.",
        (Japanese, LibrarySubtitle) => "プレイリストと保存した音楽。",
        (Korean, LibrarySubtitle) => "플레이리스트와 저장된 음악.",

        (English | Spanish | French | German | Japanese | Korean, AurafyDj) => "Aurafy DJ",

        (English, NewChat) => "New chat",
        (Spanish, NewChat) => "Nuevo chat",
        (French, NewChat) => "Nouveau chat",
        (German, NewChat) => "Neuer Chat",
        (Japanese, NewChat) => "新しいチャット",
        (Korean, NewChat) => "새 채팅",

        (English, Send) => "Send",
        (Spanish, Send) => "Enviar",
        (French, Send) => "Envoyer",
        (German, Send) => "Senden",
        (Japanese, Send) => "送信",
        (Korean, Send) => "보내기",

        (English, MessageDjPlaceholder) => "Message DJ…",
        (Spanish, MessageDjPlaceholder) => "Escribe al DJ…",
        (French, MessageDjPlaceholder) => "Écrire au DJ…",
        (German, MessageDjPlaceholder) => "Nachricht an DJ…",
        (Japanese, MessageDjPlaceholder) => "DJ にメッセージ…",
        (Korean, MessageDjPlaceholder) => "DJ에게 메시지…",

        (English, YourSet) => "Your set",
        (Spanish, YourSet) => "Tu selección",
        (French, YourSet) => "Votre sélection",
        (German, YourSet) => "Dein Set",
        (Japanese, YourSet) => "セットリスト",
        (Korean, YourSet) => "내 세트",

        (English, Updates) => "Updates",
        (Spanish, Updates) => "Actualizaciones",
        (French, Updates) => "Mises à jour",
        (German, Updates) => "Updates",
        (Japanese, Updates) => "アップデート",
        (Korean, Updates) => "업데이트",

        (English, CheckForUpdates) => "Check for updates",
        (Spanish, CheckForUpdates) => "Buscar actualizaciones",
        (French, CheckForUpdates) => "Vérifier les mises à jour",
        (German, CheckForUpdates) => "Nach Updates suchen",
        (Japanese, CheckForUpdates) => "アップデートを確認",
        (Korean, CheckForUpdates) => "업데이트 확인",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_languages_translate_non_empty() {
        let keys = [
            Text::Home,
            Text::Search,
            Text::Dj,
            Text::YourLibrary,
            Text::Settings,
            Text::LanguageLabel,
            Text::CheckForUpdates,
        ];
        for lang in Language::ALL {
            assert!(!lang.label().is_empty());
            for key in keys {
                assert!(!tr(lang, key).is_empty());
            }
        }
        assert_eq!(tr(Language::English, Text::Home), "Home");
        assert_eq!(tr(Language::Japanese, Text::Settings), "設定");
        assert_eq!(tr(Language::Korean, Text::Settings), "설정");
    }
}
