import os

import numpy as np
import pandas as pd
import neurokit2 as nk

from scipy.signal import butter, resample_poly, sosfilt
from math import gcd


class DataProcessing:
    """A class to preprocess WESAD and PulseLM data"""

    # -------------------------------------------------------------------------
    # Paths
    # -------------------------------------------------------------------------

    @staticmethod
    def load_base_data_path(input_dir: str) -> str:
        """Path to data/"""
        return os.path.join(input_dir, "../../data")

    # -------------------------------------------------------------------------
    # File I/O
    # -------------------------------------------------------------------------

    @staticmethod
    def load_from_file(data_path: str, 
                       file_type: str = 'csv',
                       sep: str = ",",
                       encoding: str = 'utf-8',
                       **kwargs) -> pd.DataFrame:
        """Load any data as long as the file type is supported."""
        if file_type in ('.csv', 'csv'):
            return pd.read_csv(data_path, sep=sep, encoding=encoding, **kwargs)
        elif file_type in ('.parquet', 'parquet'):
            return pd.read_parquet(data_path, **kwargs)
        elif file_type in ('.xlsx', 'xlsx'):
            return pd.read_excel(data_path, **kwargs)
        elif file_type in ('.json', 'json'):
            return pd.read_json(data_path, **kwargs)
        else:
            raise ValueError(f"File type '{file_type}' is not supported (yet). Choose from [csv, parquet, xlsx, json]")

    # -------------------------------------------------------------------------
    # WESAD loaders
    # -------------------------------------------------------------------------

    @staticmethod
    def load_wesad(notebook_dir: str) -> tuple[pd.DataFrame, pd.DataFrame]:
        """Load WESAD chest and wrist CSVs.

        Parameters
        ----------
        notebook_dir : str
            Current working directory of the calling notebook.

        Returns
        -------
        tuple[pd.DataFrame, pd.DataFrame]
            (chest_df, wrist_df)
        """
        base_data_path = DataProcessing.load_base_data_path(notebook_dir)

        chest_path = os.path.join(base_data_path, 'WESAD/all_subjects/chest.csv')
        wrist_path = os.path.join(base_data_path, 'WESAD/all_subjects/wrist.csv')

        chest_df = DataProcessing.load_from_file(chest_path, file_type='csv')
        wrist_df = DataProcessing.load_from_file(wrist_path, file_type='csv')

        chest_df.drop(columns=['Unnamed: 0'], errors='ignore', inplace=True)
        wrist_df.drop(columns=['Unnamed: 0'], errors='ignore', inplace=True)

        return chest_df, wrist_df

    # -------------------------------------------------------------------------
    # PulseLM loaders
    # -------------------------------------------------------------------------

    @staticmethod
    def load_pulselm_datasets(notebook_dir: str, dataset_names=None):
        """Load PulseLM dataset(s).

        Parameters
        ----------
        notebook_dir : str
            Current working directory of the calling notebook.
        dataset_names : str, list, or None
            - str  → load single dataset, returns DataFrame
            - list → load subset, returns dict
            - None → load all 16 datasets, returns dict

        Returns
        -------
        pd.DataFrame or dict[str, pd.DataFrame]
        """
        all_names = [
            'afppgecg', 'bcg', 'bidmc', 'dalia', 'earset', 'mimicperform',
            'ppgarrhythmia', 'ppgbp', 'sdb', 'sensors', 'uci', 'uqvitalsigns',
            'utsappg', 'vitaldb', 'wesad', 'wildppg'
        ]

        base_data_path = DataProcessing.load_base_data_path(notebook_dir)

        if isinstance(dataset_names, str):
            path = os.path.join(base_data_path, 'pulselm', f'{dataset_names}.parquet')
            return DataProcessing.load_from_file(path, file_type='parquet')

        names = all_names if dataset_names is None else dataset_names
        datasets = {}
        for name in names:
            try:
                path = os.path.join(base_data_path, 'pulselm', f'{name}.parquet')
                datasets[name] = DataProcessing.load_from_file(path, file_type='parquet')
                print(f"Loaded {name}: {len(datasets[name]):,} samples")
            except Exception as e:
                print(f"Error loading {name}: {e}")
        return datasets

    # -------------------------------------------------------------------------
    # Subject / label helpers
    # -------------------------------------------------------------------------

    @staticmethod
    def get_subject_ids(df: pd.DataFrame) -> np.ndarray:
        """Return unique subject IDs."""
        return df['subject'].unique()

    @staticmethod
    def get_specific_subject_df(df: pd.DataFrame, subject_id: str = 'S7') -> pd.DataFrame:
        return df[df['subject'] == subject_id]

    @staticmethod
    def filter_meaningful_labels(df: pd.DataFrame) -> pd.DataFrame:
        """Drop WESAD labels 0, 5, 6, 7 — keep 1 (baseline), 2 (stress),
        3 (amusement), 4 (meditation)."""
        return df[~df['label'].isin([0, 5, 6, 7])].copy()

    @staticmethod
    def get_specific_label(df: pd.DataFrame, label: int) -> pd.DataFrame:
        """Return rows matching a single label. Baseline = 1."""
        return df[df['label'] == label]

    @staticmethod
    def get_all_labels(df: pd.DataFrame) -> list[pd.DataFrame]:
        """Return a list of DataFrames, one per sorted unique label."""
        return [DataProcessing.get_specific_label(df, lbl)
                for lbl in sorted(df['label'].unique())]

    @staticmethod
    def check_label_order(s_df: pd.DataFrame) -> np.ndarray:
        """Return the run-start order of labels 3 and 4 for a single subject.

        Notes
        -----
        Normal   : [3, 4]
        Abnormal : [4, 3, 4]
        """
        s = s_df.loc[s_df['label'].isin([3, 4]), 'label']
        run_starts = s[s.ne(s.shift())]
        return run_starts.to_numpy()

    # -------------------------------------------------------------------------
    # Signal / column helpers
    # -------------------------------------------------------------------------

    @staticmethod
    def get_chest_measures() -> list[str]:
        return ['ACC_x', 'ACC_y', 'ACC_z', 'ECG', 'EMG', 'EDA', 'Temp', 'Resp']

    @staticmethod
    def get_signal_df(df: pd.DataFrame, signal_name: str) -> pd.DataFrame:
        """Slice a single signal column plus subject/label metadata."""
        chest_measures = DataProcessing.get_chest_measures()
        if signal_name not in chest_measures:
            raise ValueError(f"'{signal_name}' not in {chest_measures}")
        return df.loc[:, [signal_name, 'subject', 'label']]

    # -------------------------------------------------------------------------
    # Cleaning
    # -------------------------------------------------------------------------

    @staticmethod
    def clean_ecg(df: pd.DataFrame, col_name: str = 'ECG', hz: int = 700) -> pd.DataFrame:
        """Clean ECG signal using NeuroKit2."""
        df = df.copy()
        df[f'Cleaned {col_name}'] = nk.ecg_clean(df[col_name], hz)
        return df

    @staticmethod
    def clean_eda(df: pd.DataFrame, col_name: str = 'EDA', hz: int = 700) -> pd.DataFrame:
        """Clean EDA signal using NeuroKit2."""
        df = df.copy()
        df[f'Cleaned {col_name}'] = nk.eda_clean(df[col_name], hz)
        return df

    # -------------------------------------------------------------------------
    # Row reset
    # -------------------------------------------------------------------------

    @staticmethod
    def reset_rows(df: pd.DataFrame) -> pd.DataFrame:
        """Reset index to 0 per subject, then concatenate."""
        subject_ids = DataProcessing.get_subject_ids(df)
        per_subject = {}
        for sid in subject_ids:
            s_df = df[df['subject'] == sid].reset_index(drop=True)
            per_subject[sid] = s_df
        return pd.concat(per_subject.values())

    # -------------------------------------------------------------------------
    # Time conversion
    # -------------------------------------------------------------------------

    @staticmethod
    def create_time_df(df: pd.DataFrame, sampling_rate: int) -> pd.DataFrame:
        """Convert integer index to Milliseconds, Seconds, and Timedelta columns."""
        df = df.copy()
        ms_per_sample = 1000 / sampling_rate
        df['Milliseconds'] = (df.index * ms_per_sample).round(2)
        df['Seconds'] = df['Milliseconds'] / 1000.0
        df['Time'] = pd.to_timedelta(df['Seconds'], unit='s').dt.round('10ms')
        return df

    # -------------------------------------------------------------------------
    # Segmentation
    # -------------------------------------------------------------------------

    @staticmethod
    def get_segments_by_duration(df: pd.DataFrame,
                                  window_size_sec: int
                                  ) -> tuple[pd.DataFrame, pd.Series]:
        """Assign window IDs based on elapsed seconds.

        Returns
        -------
        tuple[pd.DataFrame, pd.Series]
            Annotated DataFrame and per-window sample counts.
        """
        if df.empty:
            return df.copy(), pd.Series(dtype='int64')

        out = df.copy()
        out['window_id'] = (out['Seconds'] // window_size_sec).astype(int)
        out['window_start_seconds'] = out['window_id'] * window_size_sec
        out['window_end_seconds'] = out['window_start_seconds'] + window_size_sec
        samples_per_window = out.groupby('window_id').size()
        return out, samples_per_window

    # -------------------------------------------------------------------------
    # PulseLM preprocessing pipeline (Phase 0B)
    # -------------------------------------------------------------------------

    @staticmethod
    def resample_signal(signal: np.ndarray,
                        fs_original: int,
                        fs_target: int = 125) -> np.ndarray:
        """Resample a 1-D signal from fs_original to fs_target using
        a polyphase filter (lossless rational resampling).

        Parameters
        ----------
        signal : np.ndarray
        fs_original : int
        fs_target : int
            Default 125 Hz per PulseLM spec [8].
        """
        common = gcd(fs_target, fs_original)
        up = fs_target // common
        down = fs_original // common
        return resample_poly(signal, up, down)

    @staticmethod
    def butterworth_lowpass(signal: np.ndarray,
                            fs: int = 125,
                            cutoff: float = 8.0,
                            order: int = 4) -> np.ndarray:
        """Apply fourth-order Butterworth low-pass filter at 8 Hz cutoff [8].

        Parameters
        ----------
        signal : np.ndarray
        fs : int
            Sampling rate of the (already resampled) signal.
        cutoff : float
            Cutoff frequency in Hz. Default 8 Hz per PulseLM spec [8].
        order : int
            Filter order. Default 4 per PulseLM spec [8].
        """
        nyq = fs / 2.0
        sos = butter(order, cutoff / nyq, btype='low', output='sos')
        return sosfilt(sos, signal)

    @staticmethod
    def remove_dc_offset(signal: np.ndarray) -> np.ndarray:
        """Remove DC offset via mean subtraction (per-segment) [8]."""
        return signal - np.mean(signal)

    @staticmethod
    def minmax_normalize(signal: np.ndarray) -> np.ndarray:
        """Per-segment min-max normalization to [0, 1] [8]."""
        s_min, s_max = signal.min(), signal.max()
        if s_max - s_min == 0:
            return np.zeros_like(signal)
        return (signal - s_min) / (s_max - s_min)

    @staticmethod
    def slice_into_windows(signal: np.ndarray,
                           fs: int = 125,
                           window_sec: int = 10) -> np.ndarray:
        """Slice a 1-D signal into fixed-length windows.

        Parameters
        ----------
        signal : np.ndarray
        fs : int
            Sampling rate of the signal (post-resample).
        window_sec : int
            Window duration in seconds. Default 10 s per PulseLM spec [8].

        Returns
        -------
        np.ndarray  shape (n_windows, window_length)
            Incomplete trailing window is discarded.
        """
        window_length = fs * window_sec
        n_windows = len(signal) // window_length
        return signal[:n_windows * window_length].reshape(n_windows, window_length)

    @staticmethod
    def preprocess_ppg_pipeline(signal: np.ndarray,
                                 fs_original: int,
                                 fs_target: int = 125,
                                 cutoff: float = 8.0,
                                 order: int = 4,
                                 window_sec: int = 10) -> np.ndarray:
        """Full PulseLM harmonization pipeline for a raw PPG/BVP signal [8].

        Steps
        -----
        1. Resample to 125 Hz
        2. Fourth-order Butterworth low-pass filter at 8 Hz
        3. DC offset removal
        4. Slice into 10-second windows
        5. Per-segment min-max normalization

        Parameters
        ----------
        signal : np.ndarray
            Raw 1-D waveform (e.g. WESAD wrist BVP at 64 Hz).
        fs_original : int
            Original sampling rate (e.g. 64 for WESAD wrist BVP [7]).
        fs_target : int
            Target sampling rate. Default 125 Hz [8].
        cutoff : float
            Low-pass cutoff in Hz. Default 8 Hz [8].
        order : int
            Butterworth filter order. Default 4 [8].
        window_sec : int
            Window duration in seconds. Default 10 s [8].

        Returns
        -------
        np.ndarray  shape (n_windows, fs_target * window_sec)
            Preprocessed, normalized segments ready for PaPaGei [1].
        """
        # 1. Resample
        resampled = DataProcessing.resample_signal(signal, fs_original, fs_target)

        # 2. Low-pass filter
        filtered = DataProcessing.butterworth_lowpass(resampled, fs_target, cutoff, order)

        # 3. DC offset removal
        dc_removed = DataProcessing.remove_dc_offset(filtered)

        # 4. Slice into windows
        windows = DataProcessing.slice_into_windows(dc_removed, fs_target, window_sec)

        # 5. Per-segment min-max normalization
        normalized = np.array([DataProcessing.minmax_normalize(w) for w in windows])

        return normalized