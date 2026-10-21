import polars as pl
import matplotlib.pyplot as plt

if __name__ == "__main__":
    cq_values = [0.047, 0.042, 0.032, 0.026, 0.023, 0.018, 0.0]

    w_plot = 14
    h_plot = w_plot / 2.35

    fig = plt.figure(figsize = (w_plot, h_plot))
    ax_cl = fig.add_subplot(121)
    ax_cd = fig.add_subplot(122)

    for cq in cq_values:
        cl_data = (
            pl.read_csv(f"data/cl_2d_cq_{cq:.3f}.csv", infer_schema=False)
                .rename(lambda c: c.strip())
                .with_columns(pl.all().str.strip_chars().cast(pl.Float64))
        )

        cd_data = (
            pl.read_csv(f"data/cd_2d_cq_{cq:.3f}.csv", infer_schema=False)
                .rename(lambda c: c.strip())
                .with_columns(pl.all().str.strip_chars().cast(pl.Float64))
        )

        ax_cl.plot(cl_data["aoa [deg]"], cl_data["cl"], label=f"Cq = {cq}")
        ax_cd.plot(cd_data["cl"], cd_data["cd"], label=f"Cq = {cq}")

    ax_cl.legend()

    ax_cl.set_xlabel("Angle of attack [deg]")
    ax_cl.set_ylabel("Lift coefficient (CL)")

    ax_cd.set_xlabel("Lift coefficient (CL)")
    ax_cd.set_ylabel("Drag coefficient (CD)")

    plt.show()
    