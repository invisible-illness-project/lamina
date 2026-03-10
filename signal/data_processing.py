import os 

import pandas as pd

class DataProcessing:

    def load_base_data_path(input_dir: str) -> str:
        """Path to data/"""
        return os.path.join(input_dir, "../../data")
    
    def load_from_file(data_path: str, file_type: str, **kwargs) -> pd.DataFrame:
        """Load any data as long as the file type is supported."""
        if file_type == ".csv" or file_type == "csv":
            df = pd.read_csv(data_path, **kwargs)
            return df
        else:
            return f"File type {file_type} is not support (yet). Choose from [.csv]"

    def get_s_ids(df) -> list:
        subject_ids = df['subject'].unique()
        return subject_ids
    
    def get_health_measures() -> list:
        health_measures_names = ['ACC_x', 'ACC_y', 'ACC_z', 'ECG', 'EMG', 'EDA', 'Temp', 'Resp']
        return health_measures_names

    def load_chest_df(base_dir: str, 
                      file_type: str,
                      path_to_file: str = "../../../data/WESAD/all_subjects/chest.csv"
                      ) -> pd.DataFrame:
        
        data_path = os.path.join(base_dir, path_to_file)
        df = DataProcessing.load_data(data_path, file_type)
        df.drop(['Unnamed: 0'], axis=1, inplace=True)

        return df

    def get_data_per_subject(df: pd.DataFrame, s_id: str = 'S7') -> dict:
        """Store each subject (key) and their data (value) into a dict: {s_id : data_df}. 
        If s_id exists, then return the actual subject data as well. 
        """
        per_subject = {}
        subject_ids = DataProcessing.get_s_ids(df)
        for subject_id in subject_ids:
            filt_subject = (df['subject'] == subject_id)
            subject_df = df[filt_subject]
            subject_df.reset_index(inplace=True)
            per_subject[subject_id] = subject_df

        
        if s_id in subject_ids:
            s_df = DataProcessing.get_specific_subject_df(per_subject, s_id)
            return per_subject, s_df
        
        return per_subject
    
    def get_specific_subject_df(per_subject: dict, s_id: str = 'S7') -> pd.DataFrame:
        return per_subject[s_id]
    
    def get_specific_health_measure_df(df: pd.DataFrame, health_measure_name: str) -> pd.DataFrame: 
        health_measures = DataProcessing.get_health_measures()
        if health_measure_name in health_measures:
            health_measure_df = df.loc[:, [health_measure_name]]
            return health_measure_df
        else: 
            return f"Health Measure {health_measure_name} is not support (yet). Choose from {health_measures}"
        
    def create_time_df(df: pd.DataFrame, sampling_rate: int):
        """Go from indicies to time based on sampling rate"""
        milliseconds_per_sample = (1000 / sampling_rate)

        # Elapsed time from start
        df['Milliseconds'] = (df.index * milliseconds_per_sample).round(2)

        # ✅ Numeric seconds for masks & segmentation
        df['Seconds'] = (df['Milliseconds'] / 1000.0).round(2)

        # Optional: display-only Timedelta (rounded to 10 ms so it matches two-decimal seconds)
        df['Time'] = pd.to_timedelta(df['Seconds'], unit='s').dt.round('10ms')
        return df

    def get_segments_by_duration(df, window_size_sec: int):
        # Work on a copy to avoid chained assignment issues
        out = df.copy()

        # window_id from numeric seconds
        out['window_id'] = (out['Seconds'] // window_size_sec).astype(int) + 1

        # window start/end in seconds (numeric)
        out['window_start_seconds'] = out['window_id'] * window_size_sec
        out['window_end_seconds']   = out['window_start_seconds'] + window_size_sec

        # Per-window sample counts (sanity check)
        samples_per_window = out.groupby('window_id').size()

        return out, samples_per_window

    def get_specific_label(df, label: int) -> list[pd.DataFrame]:
        """Baseline is label 0"""
        filt_label = (df['label'] == label)
        return df[filt_label]