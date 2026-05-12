using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Dynamically adjusts the Depth of Field focus distance to the ball carrier.
    /// </summary>
    public sealed class CameraFocusController : MonoBehaviour
    {
        [SerializeField] private Volume globalVolume;
        private DepthOfField dof;

        public void Initialize()
        {
            if (globalVolume != null && globalVolume.profile.TryGet<DepthOfField>(out var component))
            {
                dof = component;
            }
        }

        public void UpdateFocus(Vector3 carrierPos, Vector3 cameraPos)
        {
            if (dof == null) return;

            // Gaussian DoF uses start/end distance.
            // Ortho camera doesn't use distance for focus in the same way as perspective,
            // but URP's Gaussian DoF still works on world distance from camera plane.
            float distance = Vector3.Distance(cameraPos, carrierPos);
            
            dof.gaussianStart.value = distance - 5.0f;
            dof.gaussianEnd.value = distance + 10.0f;
        }
    }
}
