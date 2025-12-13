import os 

import pandas as pd

class DataProcessing:



    def load_data(data_path: str, file_type: str) -> pd.DataFrame:
        """Load any data as long as the file type is supported."""
        if file_type == ".csv" or file_type == "csv":
            df = pd.read_csv(data_path)
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
    
    def get_specific_health_measure_df(df: pd.DataFrame, health_measure: str) -> pd.DataFrame: 
        health_measures = DataProcessing.get_health_measures()
        if health_measure in health_measures:
            health_measure_df = df.loc[:, [health_measure]]
            return health_measure_df
        else: 
            return f"Health Measure {health_measure} is not support (yet). Choose from {health_measures}"