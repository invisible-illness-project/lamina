import os
import sys
import argparse

import numpy as np
import pandas as pd
import neurokit2 as nk

import matplotlib.pyplot as plt

from tqdm import tqdm
from datetime import datetime

from adtk.detector import AutoregressionAD, ThresholdAD

script_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.append(os.path.join(script_dir, '../signal'))

from data_processing import DataProcessing

def load_dataset(script_dir, dataset_path):
    """Load dataset from file path."""
    print("\n" + "="*50)
    print("LOAD DATASET")
    print("="*50)
    
    if not os.path.isabs(dataset_path):
        data_path = os.path.join(script_dir, dataset_path)
    else:
        data_path = dataset_path
    
    print(f"Dataset path: {data_path}")
    df = DataProcessing.load_from_file(data_path, file_type='csv')
    if 'Unnamed' in df.columns:
        df.drop(['Unnamed: 0'], axis=1, inplace=True)
    print(f"Shape: {df.shape}")
    print(f"\nPreview:\n{df.head(7)}\n")
    
    return df

if __name__ == "__main__":
    print("\n" + "="*50)
    print("ECG TO HR PIPELINE")
    print("="*50)
    
    # ============================================================
    # 1. CONFIGURATION
    # ============================================================
    base_data_path = DataProcessing.load_base_data_path(script_dir)
    print(base_data_path)
    default_dataset = os.path.join(base_data_path, 'WESAD/all_subjects/ecg_chest.csv')
    default_save_path = os.path.join(base_data_path, 'ecg_to_hr')
    default_output_name = 'data.csv'

    parser = argparse.ArgumentParser(
        description='Convert from ecg to hr.'
    )
    
    # dataset
    parser.add_argument(
        '--dataset', 
        default=default_dataset, 
        help='Path to dataset file'
        )
    
    # save_path
    parser.add_argument(
        '--save_path', 
        default=default_save_path, 
        help='Directory to save results'
        )
    
    # output_name
    parser.add_argument(
        '--output_name',
        default=default_output_name,
        help='Name for output file (e.g., )'
    )
    
    args = parser.parse_args()
    
    os.makedirs(args.save_path, exist_ok=True)

    df = load_dataset(script_dir, args.dataset)

    baseline_df = DataProcessing.get_specific_label(df, label=0)
    print(f"Shape: {baseline_df.shape}")
    print(f"\nPreview:\n{df.head(7)}\n")
