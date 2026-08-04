

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector

from ..section_models import SectionModel
from ..line_force_model import WingBuilder

class WingBuilderSettings(StormbirdSetupBaseModel):
    """
    Helper class for storing all the basic, reusable settings, for a sail, so it can be quick to 
    retrieve later. The only job is to construct a wing builder from the stored settings, but 
    where the location and foundation height may vary.

    TODO: the settings is currently done with an assumed coordinate system were the x-axis points 
    backwards, and z upwards. This should be updated to handle arbitrary coordinate systems
    """
    chord_length: float
    height: float
    section_model: SectionModel
    virtual_span_top: float = 0.0
    virtual_span_bot: float = 0.0

    def get_wing_builder(
        self, 
        x_pos: float, 
        y_pos: float, 
        foundation_height: float = 0.0,
        deck_height: float = 0.0
    ) -> WingBuilder:
        """
        Construct a wing builder from the settings
        """
        section_points = []
        line_segment_is_virtual = []
        chord_vectors = []

        chord_vector = SpatialVector(x=self.chord_length)

        non_zero_circulation_at_ends = (False, False)

        if self.virtual_span_bot > 0.0:
            
            
            section_points.append(
                SpatialVector(
                    x=x_pos, 
                    y=y_pos, 
                    z=max(deck_height, deck_height + foundation_height - self.virtual_span_top)
                )
            )
            line_segment_is_virtual.append(True)
            chord_vectors.append(chord_vector)

        section_points.append(SpatialVector(x=x_pos, y=y_pos, z=deck_height + foundation_height))
        section_points.append(SpatialVector(x=x_pos, y=y_pos, z=deck_height + foundation_height + self.height))

        
        line_segment_is_virtual.append(False)
        chord_vectors.append(chord_vector)
        chord_vectors.append(chord_vector)

        if self.virtual_span_top > 0.0:
            section_points.append(
                SpatialVector(
                    x=x_pos, 
                    y=y_pos, 
                    z=max(0.0, deck_height + foundation_height + self.height + self.virtual_span_top)
                )
            )
            line_segment_is_virtual.append(True)
            chord_vectors.append(chord_vector)
            

        return WingBuilder(
            section_points=section_points,
            chord_vectors=chord_vectors,
            section_model=self.section_model,
            non_zero_circulation_at_ends=non_zero_circulation_at_ends,
            line_segment_is_virtual=line_segment_is_virtual
        )
