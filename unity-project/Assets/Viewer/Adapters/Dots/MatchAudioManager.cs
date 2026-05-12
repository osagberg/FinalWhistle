using System;
using FinalWhistle.Viewer.Contracts;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Central manager for stylized anime sound effects triggered by MatchSim events.
    /// </summary>
    public sealed class MatchAudioManager : MonoBehaviour
    {
        [SerializeField] private AudioClip kickClip;
        [SerializeField] private AudioClip goalCheerClip;
        [SerializeField] private AudioClip whooshClip;

        private AudioSource sfxSource;
        private AudioSource crowdSource;

        public void Initialize()
        {
            if (sfxSource == null)
            {
                sfxSource = gameObject.AddComponent<AudioSource>();
                sfxSource.playOnAwake = false;
            }
            if (crowdSource == null)
            {
                crowdSource = gameObject.AddComponent<AudioSource>();
                crowdSource.playOnAwake = false;
                crowdSource.loop = true;
                crowdSource.volume = 0.2f;
            }
        }

        public void PlayKick() => sfxSource.PlayOneShot(kickClip, 0.8f);
        public void PlayWhoosh() => sfxSource.PlayOneShot(whooshClip, 0.6f);
        
        public void PlayGoal()
        {
            sfxSource.PlayOneShot(goalCheerClip, 1.0f);
            // Swell the ambient crowd volume
            crowdSource.volume = 0.5f;
            Invoke(nameof(ResetCrowdVolume), 4.0f);
        }

        private void ResetCrowdVolume()
        {
            crowdSource.volume = 0.2f;
        }

        public void StartCrowdAmbience()
        {
            if (crowdSource != null && !crowdSource.isPlaying)
            {
                crowdSource.Play();
            }
        }
    }
}
